//! Wire-level pairing protocol for authenticated local settings transfers.
//!
//! Both the desktop application and the ESP32 firmware use this crate so the
//! version, algorithm suite, framing, and key schedule stay identical.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

/// Largest plaintext accepted for a handshake or status frame.
pub const MAX_HANDSHAKE_PAYLOAD: usize = 256;

pub mod channel;
pub mod pake;
pub mod protocol;
#[cfg(feature = "std")]
pub mod transport;
pub mod window;

use core::fmt;

/// Failure of a pairing step before any application payload is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// The code is not six ASCII digits.
    InvalidCode,
    /// The peer requested an unknown protocol version.
    UnsupportedVersion,
    /// The peer requested an unknown algorithm suite.
    UnsupportedSuite,
    /// A frame was malformed, truncated, or arrived out of order.
    InvalidFrame,
    /// The encoded payload exceeds the negotiated maximum.
    PayloadTooLarge,
    /// The code is expired, consumed, or has no attempts left.
    WindowUnavailable,
    /// The code or authenticated message did not verify.
    AuthenticationFailed,
    /// Randomness, key derivation, or authenticated encryption failed.
    ChannelFailed,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidCode => "The pairing code must contain six digits",
            Self::UnsupportedVersion => "Unsupported pairing protocol version",
            Self::UnsupportedSuite => "Unsupported pairing algorithm suite",
            Self::InvalidFrame => "Malformed pairing frame",
            Self::PayloadTooLarge => "Pairing payload exceeds the negotiated limit",
            Self::WindowUnavailable => "The pairing code is no longer available",
            Self::AuthenticationFailed => "Pairing authentication failed",
            Self::ChannelFailed => "The protected transfer failed",
        };
        formatter.write_str(message)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProtocolError {}

/// Milliseconds since the first call on this process.
#[cfg(feature = "std")]
pub fn now_ms() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn errors_have_stable_human_readable_messages() {
        assert_eq!(
            ProtocolError::UnsupportedVersion.to_string(),
            "Unsupported pairing protocol version"
        );
        assert_eq!(
            ProtocolError::AuthenticationFailed.to_string(),
            "Pairing authentication failed"
        );
    }
}
