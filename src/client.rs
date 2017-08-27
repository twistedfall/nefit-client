use std::time;

use libstrophe::jid;

use crate::{Communicator, Cryptor, Result};

const ACCESSKEY_PREFIX: &str = "Ct7ZR03b_";
const RRC_CONTACT_PREFIX: &str = "rrccontact_";
const RRC_GATEWAY_PREFIX: &str = "rrcgateway_";

#[derive(Debug, Clone)]
pub struct Client {
	serial: String,
	access_key: String,
	host: String,
	cryptor: Cryptor,
}

impl Client {
	pub fn new(serial: impl Into<String>, access_key: impl Into<String>, password: impl AsRef<[u8]>) -> Self {
		Self::new_with_host("wa2-mz36-qrmzh6.bosch.de", serial, access_key, password)
	}

	pub fn new_with_host(
		host: impl Into<String>,
		serial: impl Into<String>,
		access_key: impl Into<String>,
		password: impl AsRef<[u8]>,
	) -> Self {
		let access_key = access_key.into();
		let cryptor = Cryptor::new(&access_key, password);
		Self {
			serial: serial.into(),
			access_key,
			host: host.into(),
			cryptor,
		}
	}

	pub fn connect(self) -> Result<Communicator> {
		let ctx = libstrophe::Context::new_with_default_logger();
		let from = jid::jid_new(Some(&format!("{}{}", RRC_CONTACT_PREFIX, self.serial)), &self.host, None)
			.expect("Cannot create 'from' jid");
		let to =
			jid::jid_new(Some(&format!("{}{}", RRC_GATEWAY_PREFIX, self.serial)), &self.host, None).expect("Cannot create 'to' jid");
		let mut conn = libstrophe::Connection::new(ctx);
		conn
			.set_flags(libstrophe::ConnectionFlags::MANDATORY_TLS)
			.expect("Cannot set libstrophe flags");
		conn.set_keepalive(time::Duration::from_secs(10), time::Duration::from_secs(10));
		conn.set_jid(&from);
		conn.set_pass(format!("{}{}", ACCESSKEY_PREFIX, self.access_key));
		Communicator::new(conn, self.cryptor, self.host, from, to)
	}
}
