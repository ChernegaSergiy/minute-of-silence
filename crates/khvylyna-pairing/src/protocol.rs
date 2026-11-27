//! Versioned, length-prefixed frames shared by every Khvylyna pairing peer.

use crate::{MAX_HANDSHAKE_PAYLOAD, ProtocolError};

/// Bytes that open every pairing frame.
pub const MAGIC: [u8; 8] = *b"KHVPAIR1";
/// Protocol version carried in the first frame of every connection.
pub const VERSION: u8 = 1;
/// Algorithm suite: CPace over Ristretto255, HKDF-SHA256, AES-256-GCM.
pub const SUITE_CPACE_RISTRETTO_AES256_GCM: u8 = 1;
/// Magic, version, suite, kind, and big-endian payload length.
pub const HEADER_LEN: usize = 13;
/// Largest payload accepted on desktop and Android peers (16 KiB).
pub const DESKTOP_MAX_PAYLOAD: u16 = 16 * 1024;
/// Largest payload accepted by the ESP32 firmware.
pub const DEVICE_MAX_PAYLOAD: u16 = 4 * 1024;

/// Receiver does not support the requested protocol version.
pub const ERROR_UNSUPPORTED_VERSION: u8 = 1;
/// Receiver does not support the requested algorithm suite.
pub const ERROR_UNSUPPORTED_SUITE: u8 = 2;
/// Pairing code expired, was consumed, or exhausted its attempts.
pub const ERROR_WINDOW_UNAVAILABLE: u8 = 3;
/// A frame was malformed or appeared in the wrong state.
pub const ERROR_INVALID_MESSAGE: u8 = 4;
/// The payload exceeds the negotiated maximum.
pub const ERROR_PAYLOAD_TOO_LARGE: u8 = 5;

/// Kind of a pairing frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// Sender offers its version, suite, and maximum payload.
    Hello = 1,
    /// Receiver answers with the negotiated maximum payload.
    HelloAck = 2,
    /// Sender's CPace step-one packet.
    PakeInit = 3,
    /// Receiver's CPace step-two packet.
    PakeResponse = 4,
    /// Sender proves knowledge of the code with an encrypted marker.
    Confirm = 5,
    /// Sender's single encrypted settings payload.
    Data = 6,
    /// Receiver's single encrypted accept/reject result.
    Ack = 7,
    /// Receiver's plaintext rejection with a one-byte reason.
    ErrorFrame = 8,
}

impl TryFrom<u8> for FrameKind {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::HelloAck),
            3 => Ok(Self::PakeInit),
            4 => Ok(Self::PakeResponse),
            5 => Ok(Self::Confirm),
            6 => Ok(Self::Data),
            7 => Ok(Self::Ack),
            8 => Ok(Self::ErrorFrame),
            _ => Err(ProtocolError::InvalidFrame),
        }
    }
}

/// Parsed fixed-size frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// Protocol version from the frame.
    pub version: u8,
    /// Algorithm suite from the frame.
    pub suite: u8,
    /// Frame kind.
    pub kind: FrameKind,
    /// Payload length in bytes.
    pub len: usize,
}

/// Parses and enforces the version, suite, magic, and payload limit of a header.
pub fn parse_header(buf: &[u8], negotiated_max: u16) -> Result<FrameHeader, ProtocolError> {
    if buf.len() < HEADER_LEN || buf[..MAGIC.len()] != MAGIC {
        return Err(ProtocolError::InvalidFrame);
    }
    let version = buf[MAGIC.len()];
    let suite = buf[MAGIC.len() + 1];
    let kind = FrameKind::try_from(buf[MAGIC.len() + 2])?;
    let len = u16::from_be_bytes([buf[MAGIC.len() + 3], buf[MAGIC.len() + 4]]) as usize;

    if version != VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }
    if suite != SUITE_CPACE_RISTRETTO_AES256_GCM {
        return Err(ProtocolError::UnsupportedSuite);
    }
    let limit = match kind {
        FrameKind::Hello
        | FrameKind::HelloAck
        | FrameKind::PakeInit
        | FrameKind::PakeResponse
        | FrameKind::Confirm
        | FrameKind::Ack
        | FrameKind::ErrorFrame => MAX_HANDSHAKE_PAYLOAD,
        FrameKind::Data => crate::channel::encrypted_len(negotiated_max as usize),
    };
    if len > limit {
        return Err(ProtocolError::PayloadTooLarge);
    }
    Ok(FrameHeader {
        version,
        suite,
        kind,
        len,
    })
}

/// Writes a frame header into `out`, which must have at least [`HEADER_LEN`] bytes.
pub fn write_header(
    kind: FrameKind,
    payload_len: usize,
    out: &mut [u8],
) -> Result<(), ProtocolError> {
    let len = u16::try_from(payload_len).map_err(|_| ProtocolError::PayloadTooLarge)?;
    if out.len() < HEADER_LEN {
        return Err(ProtocolError::InvalidFrame);
    }
    out[..MAGIC.len()].copy_from_slice(&MAGIC);
    out[MAGIC.len()] = VERSION;
    out[MAGIC.len() + 1] = SUITE_CPACE_RISTRETTO_AES256_GCM;
    out[MAGIC.len() + 2] = kind as u8;
    out[MAGIC.len() + 3..HEADER_LEN].copy_from_slice(&len.to_be_bytes());
    Ok(())
}

/// Encodes a complete frame into an owned buffer.
pub fn encode_frame(
    kind: FrameKind,
    payload: &[u8],
    out: &mut alloc::vec::Vec<u8>,
) -> Result<(), ProtocolError> {
    if payload.len() > u16::MAX as usize {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let start = out.len();
    out.resize(start + HEADER_LEN, 0);
    write_header(kind, payload.len(), &mut out[start..])?;
    out.extend_from_slice(payload);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_keeps_version_suite_kind_and_length() {
        let mut bytes = [0u8; HEADER_LEN];
        write_header(FrameKind::PakeInit, 48, &mut bytes).expect("header");
        let parsed = parse_header(&bytes, DESKTOP_MAX_PAYLOAD).expect("parse");
        assert_eq!(parsed.version, VERSION);
        assert_eq!(parsed.suite, SUITE_CPACE_RISTRETTO_AES256_GCM);
        assert_eq!(parsed.kind, FrameKind::PakeInit);
        assert_eq!(parsed.len, 48);
    }

    #[test]
    fn unknown_version_and_suite_are_rejected() {
        let mut bytes = [0u8; HEADER_LEN];
        write_header(FrameKind::Hello, 2, &mut bytes).expect("header");
        bytes[MAGIC.len()] = VERSION + 1;
        assert_eq!(
            parse_header(&bytes, DESKTOP_MAX_PAYLOAD),
            Err(ProtocolError::UnsupportedVersion)
        );
        bytes[MAGIC.len()] = VERSION;
        bytes[MAGIC.len() + 1] = SUITE_CPACE_RISTRETTO_AES256_GCM + 1;
        assert_eq!(
            parse_header(&bytes, DESKTOP_MAX_PAYLOAD),
            Err(ProtocolError::UnsupportedSuite)
        );
    }

    #[test]
    fn limits_are_enforced_before_the_payload_is_read() {
        let mut bytes = [0u8; HEADER_LEN];
        write_header(
            FrameKind::Data,
            crate::channel::encrypted_len(DEVICE_MAX_PAYLOAD as usize) + 1,
            &mut bytes,
        )
        .expect("header");
        assert_eq!(
            parse_header(&bytes, DEVICE_MAX_PAYLOAD),
            Err(ProtocolError::PayloadTooLarge)
        );
    }
}
