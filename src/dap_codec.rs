//! A tokio-util Codec implementation for DAP messages.
//!
//! # Base protocol
//!
//! The base protocol exchanges messages that consist of a header and a content
//! part (comparable to HTTP). The header and content part are separate by a
//! \r\n (carriage return, line feed).
//!
//! # Header part
//!
//! The header part consists of header fields. Each header field is comprised
//! of a key and a value, separated by a `: ` (a colon and a space). Each
//! header field is terminated by `\r\n`.
//!
//! Since both the last header field and the overall header itself are each
//! terminated with `\r\n`, and since the header is mandatory, the content part
//! of a message is always preceded (and uniquely identified) by two `\r\n`
//! sequences.
//!
//! Currently, only a single header field is supported and required:
//!
//! - `Content-Length` (number): The length of the content part in bytes. This
//! header is required.
//!
//! The header part is encoded using the `ASCII` encoding. This includes the
//! `\r\n` separating the header and content part.
//!
//! # Content part
//!
//! The content part contains the actual content of the message. The content
//! part of a message uses JSON to describe requests, responses, and events.
//!
//! # Example
//!
//! This example shows the JSON for the DAP `next` request:
//!
//! ```txt
//! Content-Length: 119\r\n
//! \r\n
//! {
//!     "seq": 153,
//!     "type": "request",
//!     "command": "next",
//!     "arguments": {
//!         "threadId": 3
//!     }
//! }
//! ```

use std::io::BufRead;
use std::{collections::HashMap, io};

use crate::adapter::Adapter;
use crate::dap_types::AdapterMessage;
use bytes::{Buf, BytesMut};
use itertools::Itertools;
use serde::Deserialize;
use tokio_util::codec::Decoder;

const CONTENT_LENGTH_LEN: usize = "Content-Length: ".len();

fn find_delim(src: &[u8], first: u8, second: u8) -> Option<usize> {
    src.iter()
        .tuple_windows()
        .position(|(elem, next)| elem == &first && next == &second)
}

pub struct DapCodec {}

impl Decoder for DapCodec {
    type Item = AdapterMessage;

    type Error = DapCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Parse content length header
        // TODO: Reduce slice sizes to safe performance

        // TODO: Support multiple headers
        let sep = find_delim(&src[..], b':', b' ').ok_or(DapCodecError::MalformedHeader)?;
        // Get key
        let mut key = Vec::with_capacity(sep);
        key.copy_from_slice(&src[..sep]);
        let key = String::from_utf8(key).map_err(|_| DapCodecError::MalformedHeader)?;
        if key != "Content-Length" {
            return Err(DapCodecError::UnrecognizedHeader(key));
        }
        // Get value
        let end =
            find_delim(&src[sep + 2..], b'\r', b'\n').ok_or(DapCodecError::MalformedHeader)?;
        let mut value = Vec::with_capacity(end - sep + 2);
        value.copy_from_slice(&src[sep + 2..end]);
        let value = String::from_utf8(value).map_err(|_| DapCodecError::MalformedHeader)?;
        let content_len = value
            .parse::<usize>()
            .map_err(|_| DapCodecError::MalformedHeader)?;

        // Check length
        if src.len() - end < content_len {
            // The full message has not yet arrived
            //
            // We reserve more space in the buffer. This is not strictly
            // necessary, but is a good idea performance-wise.
            src.reserve(content_len - src.len());

            // Inform the frame that we need more bytes
            return Ok(None);
        }

        // Retrieve payload and advance stream to no longer contain this frame
        let payload = src[end + 2..end + 2 + content_len].to_vec();
        src.advance(end + 2 + content_len);

        // Convert payload to an AdapterMessage
        match serde_json::from_slice::<AdapterMessage>(&payload) {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => Err(DapCodecError::ParseError(e)),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DapCodecError {
    #[error("IO error: {0}")]
    IO(io::Error),
    #[error("Malformed header")]
    MalformedHeader,
    #[error("Parse error: {0}")]
    ParseError(serde_json::Error),
    #[error("Unrecognized header: {0}")]
    UnrecognizedHeader(String),
}

impl From<io::Error> for DapCodecError {
    fn from(e: io::Error) -> Self {
        Self::IO(e)
    }
}
