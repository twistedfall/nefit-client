use std::borrow::Cow;
use std::fmt;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
#[error("CryptError: {0}")]
pub struct CryptError(pub String);

#[derive(Debug, ThisError)]
#[error("CommunicationError: {0}")]
pub struct CommunicationError(pub Cow<'static, str>);

#[derive(Debug, ThisError)]
#[error("DeserializeError: {0}")]
pub struct DeserializeError(pub String);

impl serde::de::Error for DeserializeError {
	fn custom<T: fmt::Display>(msg: T) -> Self {
		DeserializeError(msg.to_string())
	}
}

pub use anyhow::Result;
