use std::sync::{Arc, RwLock, mpsc};
use std::{mem, thread, time};

use libstrophe::{Connection, ConnectionEvent, Context, HandlerResult, Stanza};
use log::{debug, error};
use serde::de::IntoDeserializer;

use crate::command::{get, put};
use crate::error::{CommunicationError, DeserializeError, Result};
use crate::{Command, Cryptor, RawCommand, RawCommandResult, command};

#[derive(Debug)]
enum CommunicatorStatus {
	Connecting,
	Connected,
	Idle,
	WaitingForReply((Option<&'static str>, Option<&'static str>, Option<&'static str>)),
	#[expect(dead_code)]
	WaitingForReplyNoRespond,
	ReplyReady(Option<String>),
	Disconnecting,
	Disconnected,
}

const QUERY: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>');

#[derive(Debug)]
pub struct Communicator {
	status: Arc<RwLock<CommunicatorStatus>>,
	thread_join: Option<thread::JoinHandle<Result<()>>>,
	to_thread: mpsc::Sender<RawCommand>,
	from_thread: mpsc::Receiver<Box<Result<RawCommandResult>>>,
}

impl Communicator {
	pub fn new(
		conn: Connection<'static, 'static>,
		cryptor: Cryptor,
		host: String,
		from: String,
		to: String,
	) -> Result<Communicator> {
		let status = Arc::new(RwLock::new(CommunicatorStatus::Connecting));
		let (to_master, from_thread) = mpsc::channel();
		let (to_thread, from_master) = mpsc::channel();
		let main = {
			let status = status.clone();
			move || -> Result<()> {
				let connect_cb = {
					let status = status.clone();
					move |ctx: &Context, _conn: &mut Connection, evt: ConnectionEvent| match evt {
						ConnectionEvent::Connect | ConnectionEvent::RawConnect => {
							*status.write().expect("Cannot lock RwLock for writing") = CommunicatorStatus::Connected
						}
						ConnectionEvent::Disconnect(..) => {
							*status.write().expect("Cannot lock RwLock for writing") = CommunicatorStatus::Disconnected;
							ctx.stop();
						}
					}
				};

				let reply_handler = {
					let status = status.clone();
					move |_ctx: &Context, _conn: &mut Connection, stanza: &Stanza| {
						let mut status = status.write().expect("Cannot lock RwLock for writing");
						match *status {
							CommunicatorStatus::WaitingForReply(..) => *status = CommunicatorStatus::ReplyReady(stanza.body()),
							CommunicatorStatus::WaitingForReplyNoRespond => *status = CommunicatorStatus::Idle,
							_ => unreachable!("Invalid internal state"),
						}
						HandlerResult::RemoveHandler
					}
				};

				let mut conn = conn;
				let timer_granularity = time::Duration::from_millis(100);
				conn.timed_handler_add(
					{
						let status = Arc::clone(&status);
						move |_, conn| {
							match *status.write().expect("Cannot lock RwLock for writing") {
								ref mut status @ CommunicatorStatus::Connected => {
									*status = CommunicatorStatus::Idle;
									//								let filters = RawCommand::Ping.get_reply_stanza_filters();
									//								conn.handler_add(&reply_handler, filters.0, filters.1, filters.2);
									conn.send(&Stanza::new_presence());
								}
								ref mut status @ CommunicatorStatus::Idle => {
									if let Some(command) = match from_master.try_recv() {
										Ok(command) => Some(command),
										Err(mpsc::TryRecvError::Disconnected) => Some(RawCommand::Disconnect),
										Err(mpsc::TryRecvError::Empty) => None,
									} {
										let res = Communicator::process_command(command.clone(), conn, status, &to, &from, &cryptor);
										if let Err(e) = res {
											error!("Error processing command: {command}, error: {e}");
										}
										if let CommunicatorStatus::WaitingForReply(ref filters) = *status {
											conn.handler_add(reply_handler.clone(), filters.0, filters.1, filters.2);
										}
									}
								}
								ref mut status @ CommunicatorStatus::ReplyReady(..) => {
									let status = mem::replace(status, CommunicatorStatus::Idle);
									if let CommunicatorStatus::ReplyReady(reply) = status {
										to_master
											.send(Box::new(Communicator::process_reply(reply, &cryptor)))
											.unwrap(); //fixme
									}
								}
								CommunicatorStatus::Disconnected => conn.disconnect(),
								_ => {}
							}
							HandlerResult::KeepHandler
						}
					},
					timer_granularity,
				);
				let mut ctx = conn.connect_client(Some(&host), None, connect_cb).unwrap();
				ctx.run();
				Ok(())
			}
		};
		let thread_join = thread::spawn(main);
		Ok(Communicator {
			status,
			thread_join: Some(thread_join),
			to_thread,
			from_thread,
		})
	}

	fn create_raw_message(to: &str, from: &str, body: &str) -> String {
		let mut st = Stanza::new_message(Some("chat"), None, Some(to));
		st.set_from(from).expect("Cannot set from");
		st.set_body(body).expect("Cannot set body");
		st.to_string().replace('\r', "&#13;")
	}

	fn process_command(
		command: RawCommand,
		conn: &mut Connection,
		status: &mut CommunicatorStatus,
		to: &str,
		from: &str,
		cryptor: &Cryptor,
	) -> Result<()> {
		debug!("*** Received command = {command:#?}");
		let filters = command.get_reply_stanza_filters();
		match command {
			RawCommand::Ping => {
				*status = CommunicatorStatus::WaitingForReply(filters);
				conn.send(&Stanza::new_presence());
			}
			RawCommand::Disconnect => {
				*status = CommunicatorStatus::Disconnecting;
				conn.disconnect()
			}

			RawCommand::Get(url) => {
				let body = format!(
					"GET {} HTTP/1.1\r\nUser-Agent: NefitEasy\r\n\r\n",
					percent_encoding::utf8_percent_encode(&url, QUERY)
				);
				conn.send_raw(Communicator::create_raw_message(to, from, &body));
				*status = CommunicatorStatus::WaitingForReply(filters);
			}
			RawCommand::Put(url, value) => {
				let val = serde_json::to_string(&command::ValuePut { value })?;
				let enc_value = cryptor.encrypt(val)?;
				let body = format!(
					"PUT {} HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nUser-Agent: NefitEasy\r\n\r\n{}",
					percent_encoding::utf8_percent_encode(&url, QUERY),
					enc_value.len(),
					enc_value,
				);
				conn.send_raw(Communicator::create_raw_message(to, from, &body));
				*status = CommunicatorStatus::WaitingForReply(filters);
			}
		};
		Ok(())
	}

	fn process_reply(response: Option<String>, cryptor: &Cryptor) -> Result<RawCommandResult> {
		match response {
			None => Ok(RawCommandResult::Empty),
			Some(response) => {
				let mut headers = [httparse::EMPTY_HEADER; 5];
				let parse_res = {
					let mut parser = httparse::Response::new(&mut headers);
					let out = parser.parse(response.as_bytes())?;
					if let Some(code) = parser.code {
						if !(200..300).contains(&code) {
							return Err(
								CommunicationError(
									parser
										.reason
										.map(|x| x.to_string().into())
										.unwrap_or("Unspecified error".into()),
								)
								.into(),
							);
						}
					}
					out
				};
				match parse_res {
					httparse::Status::Complete(body_start) => {
						for header in headers.iter().take_while(|x| !x.name.is_empty()) {
							if header.name == "Content-Type" {
								if header.value == b"application/json" {
									let body = &response[body_start..];
									return if body.is_empty() {
										Ok(RawCommandResult::Empty)
									} else {
										Ok(RawCommandResult::Json(cryptor.decrypt(body)?))
									};
								} else {
									panic!("Unknown content-type: {}", String::from_utf8_lossy(header.value))
								}
							}
						}
						panic!("Content-type not found")
					}
					_ => panic!("Error while parsing response"),
				}
			}
		}
	}

	pub fn is_ready(&self) -> bool {
		matches!(
			*self.status.read().expect("Cannot lock RwLock for reading"),
			CommunicatorStatus::Idle
		)
	}

	pub fn send_raw_with_reply(&self, command: RawCommand) -> Result<RawCommandResult> {
		self.to_thread.send(command)?;
		*self.from_thread.recv()?
	}

	pub fn send_raw(&self, command: RawCommand) -> Result<()> {
		Ok(self.to_thread.send(command)?)
	}

	pub fn send<RE: serde::de::DeserializeOwned>(&self, command: Command<RE>) -> Result<RE> {
		match self.send_raw_with_reply(command.into())? {
			RawCommandResult::Empty => RE::deserialize(().into_deserializer()).map_err(|e: DeserializeError| e.into()),
			RawCommandResult::Json(res) => Ok(serde_json::from_str(&res)?),
		}
	}

	pub fn ping(&self) -> Result<RawCommandResult> {
		self.send_raw_with_reply(RawCommand::Ping)
	}

	pub fn disconnect(self) {}

	pub fn system_pressure(&self) -> Result<f64> {
		Ok(self.send(get::system_pressure)?.value)
	}

	pub fn display_code(&self) -> Result<String> {
		Ok(self.send(get::display_code)?.value)
	}

	pub fn cause_code(&self) -> Result<f64> {
		Ok(self.send(get::cause_code)?.value)
	}

	pub fn latitude(&self) -> Result<String> {
		Ok(self.send(get::latitude)?.value)
	}

	pub fn longitude(&self) -> Result<String> {
		Ok(self.send(get::longitude)?.value)
	}

	pub fn outdoor_temp(&self) -> Result<f64> {
		Ok(self.send(get::outdoor_temp)?.value)
	}

	pub fn supply_temp(&self) -> Result<f64> {
		Ok(self.send(get::supply_temp)?.value)
	}

	pub fn user_mode(&self) -> Result<String> {
		Ok(self.send(get::user_mode)?.value)
	}

	pub fn status(&self) -> Result<command::UiUpdate> {
		Ok(self.send(get::status)?.value)
	}

	pub fn gas_usage_entry_count(&self) -> Result<usize> {
		Ok(self.send(get::gas_usage_entry_count)?.value as usize)
	}

	pub fn gas_usage_page_count(&self) -> Result<usize> {
		Ok((self.send(get::gas_usage_entry_count)?.value / command::GAS_USAGE_ENTRIES_PER_PAGE as f64).ceil() as usize)
	}

	pub fn gas_usage_page(&self, page_num: usize) -> Result<Vec<command::Recording>> {
		Ok(self
			.send(get::gas_usage_page(page_num))?
			.value
			.into_iter()
			.filter_map(command::Recording::from_raw)
			.collect())
	}

	pub fn set_manual_temp_override(&self, temp: f64) -> Result<()> {
		self.send(put::set_manual_temp_override(temp))
	}

	pub fn set_temp_room_manual(&self, temp: f64) -> Result<()> {
		self.send(put::set_temp_room_manual(temp))
	}

	pub fn enable_manual_temp_override(&self, enable: bool) -> Result<()> {
		self.send(put::enable_manual_temp_override(enable))
	}
}

impl Drop for Communicator {
	fn drop(&mut self) {
		let res = self.send_raw(RawCommand::Disconnect);
		if let Err(e) = res {
			error!("Cannot send Disconnect command, skipping: {e}");
		}
		self
			.thread_join
			.take()
			.unwrap()
			.join()
			.expect("Cannot join worker thread")
			.map_err(|e| panic!("Error in Communicator main thread: {e}"))
			.unwrap();
	}
}
