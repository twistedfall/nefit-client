//! # Communication library for Nefit (Bosch) gas boilers
//!
//! A word of warning: the last time I used this code in production was in March 2023, so it could've code-rotten since.
//!
//! Not all endpoints are implemented, the focus was mostly on reading the data. I will gladly accept PRs.
//!
//! ## Nefit API specifics
//!
//! The communication method is quite peculiar because it's essentially HTTP with custom encryption over XMPP. The library
//! connects to Bosch XMPP server and then exchanges encrypted messages which are HTTP requests and responses.
//!
//! # Example
//!
//! ```no_run
//! let cl = nefit_client::Client::new("<SERIAL_NUMBER>", "<ACCESS_KEY>", "<PASSWORD>");
//! let cm = cl.connect().unwrap();
//! dbg!(cm.status().unwrap());
//! dbg!(cm.outdoor_temp().unwrap());
//! dbg!(cm.system_pressure().unwrap());
//! dbg!(cm.supply_temp().unwrap());
//! ```
//!
//! # Useful links
//! * https://github.com/robertklep/nefit-easy-core
//! * https://gathering.tweakers.net/forum/list_messages/1659309/0
//! * https://www.domoticz.com/forum/viewtopic.php?t=9653

pub use error::{CommunicationError, CryptError, DeserializeError, Result};

pub use crate::client::Client;
pub use crate::command::{Command, RawCommand, RawCommandResult};
pub use crate::communicator::Communicator;
use crate::cryptor::Cryptor;

mod client;
pub mod command;
mod communicator;
mod cryptor;
mod error;
