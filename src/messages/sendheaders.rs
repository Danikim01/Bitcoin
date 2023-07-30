use crate::messages::constants::commands::SENDHEADERS;
use crate::messages::{MessageHeader, Serialize};

/// Struct that represents the SendHeaders message
#[derive(Debug, Clone)]
pub struct SendHeaders {
    _message_header: MessageHeader,
}

impl SendHeaders {
    /// Creates a new `SendHeaders` message with the default values.
    pub fn new() -> Self {
        Self {
            _message_header: MessageHeader::default(),
        }
    }
}

impl Serialize for SendHeaders {
    fn serialize(&self) -> std::io::Result<Vec<u8>> {
        let message = self.build_message(SENDHEADERS, None)?;
        Ok(message)
    }
}
