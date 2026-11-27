//! Balanced CPace exchange for a six-digit, single-use pairing code.

use alloc::{format, string::String, vec::Vec};
use pake_cpace_embedded::{CPace, STEP1_PACKET_BYTES, STEP2_PACKET_BYTES, Step1Out};
use rand_core::{CryptoRng, RngCore};
use zeroize::Zeroizing;

use crate::{ProtocolError, channel::SessionSecret};

/// CPace identifier of the party entering the code.
pub const ID_INITIATOR: &[u8] = b"khvylyna-pairing-v1 initiator";
/// CPace identifier of the party displaying the code.
pub const ID_RESPONDER: &[u8] = b"khvylyna-pairing-v1 responder";
/// Length of the initiator's CPace packet.
pub const STEP1_LEN: usize = STEP1_PACKET_BYTES;
/// Length of the responder's CPace packet.
pub const STEP2_LEN: usize = STEP2_PACKET_BYTES;

/// Returns whether `code` is exactly six ASCII digits.
pub fn is_valid_code(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
}

/// Generates an unbiased six-digit code with the supplied CSPRNG.
pub fn generate_code<R: RngCore + CryptoRng>(
    rng: &mut R,
) -> Result<Zeroizing<String>, ProtocolError> {
    const CODE_SPACE: u32 = 1_000_000;
    const LIMIT: u32 = u32::MAX - (u32::MAX % CODE_SPACE);
    loop {
        let mut bytes = [0u8; 4];
        rng.try_fill_bytes(&mut bytes)
            .map_err(|_| ProtocolError::ChannelFailed)?;
        let value = u32::from_le_bytes(bytes);
        if value < LIMIT {
            return Ok(Zeroizing::new(format!("{:06}", value % CODE_SPACE)));
        }
    }
}

fn additional_data(max_payload: u16) -> [u8; 2] {
    max_payload.to_be_bytes()
}

/// Sender state after creating the first CPace packet.
pub struct SenderExchange {
    exchange: Step1Out,
    max_payload: u16,
}

impl SenderExchange {
    /// Starts an exchange using the code displayed by the receiver.
    pub fn start<R: RngCore + CryptoRng>(
        code: &str,
        max_payload: u16,
        rng: &mut R,
    ) -> Result<(Self, Vec<u8>), ProtocolError> {
        if !is_valid_code(code) {
            return Err(ProtocolError::InvalidCode);
        }
        if max_payload == 0 {
            return Err(ProtocolError::PayloadTooLarge);
        }
        let ad = additional_data(max_payload);
        let exchange = CPace::step1_with_rng(
            code.as_bytes(),
            ID_INITIATOR,
            ID_RESPONDER,
            Some(ad.as_slice()),
            rng,
        )
        .map_err(|_| ProtocolError::ChannelFailed)?;
        let packet = exchange.packet().to_vec();
        Ok((
            Self {
                exchange,
                max_payload,
            },
            packet,
        ))
    }

    /// Derives the session secret after receiving the responder's packet.
    pub fn finish(self, code: &str, response: &[u8]) -> Result<SessionSecret, ProtocolError> {
        if !is_valid_code(code) {
            return Err(ProtocolError::InvalidCode);
        }
        if self.max_payload == 0 {
            return Err(ProtocolError::PayloadTooLarge);
        }
        let response: [u8; STEP2_LEN] = response
            .try_into()
            .map_err(|_| ProtocolError::InvalidFrame)?;
        let keys = self
            .exchange
            .step3(&response)
            .map_err(|_| ProtocolError::AuthenticationFailed)?;
        Ok(SessionSecret::new(keys.k1, keys.k2))
    }
}

/// Receiver state that still has to verify the sender's confirmation.
pub struct PairingAttempt {
    secret: SessionSecret,
}

/// Receiver state after the sender proved knowledge of the pairing code.
pub struct ConfirmedAttempt(SessionSecret);

impl PairingAttempt {
    /// Verifies the sender's confirmation and releases a finish-only token.
    pub fn verify_confirm(self, message: &[u8]) -> Result<ConfirmedAttempt, ProtocolError> {
        self.secret.open_confirm(message)?;
        Ok(ConfirmedAttempt(self.secret))
    }
}

impl ConfirmedAttempt {
    /// Returns the established session secret.
    pub fn into_secret(self) -> SessionSecret {
        self.0
    }
}

/// Responds to an initiator packet using the receiver's displayed code.
pub fn receiver_response<R: RngCore + CryptoRng>(
    code: &str,
    request: &[u8],
    max_payload: u16,
    rng: &mut R,
) -> Result<(PairingAttempt, Vec<u8>), ProtocolError> {
    if !is_valid_code(code) {
        return Err(ProtocolError::InvalidCode);
    }
    if max_payload == 0 {
        return Err(ProtocolError::PayloadTooLarge);
    }
    let request: [u8; STEP1_LEN] = request
        .try_into()
        .map_err(|_| ProtocolError::InvalidFrame)?;
    let ad = additional_data(max_payload);
    let exchange = CPace::step2_with_rng(
        &request,
        code.as_bytes(),
        ID_INITIATOR,
        ID_RESPONDER,
        Some(ad.as_slice()),
        rng,
    )
    .map_err(|_| ProtocolError::AuthenticationFailed)?;
    let keys = exchange.shared_keys();
    Ok((
        PairingAttempt {
            secret: SessionSecret::new(keys.k1, keys.k2),
        },
        exchange.packet().to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestRng {
        bytes: alloc::vec::Vec<u8>,
        cursor: usize,
    }

    impl TestRng {
        fn new(bytes: &[u8]) -> Self {
            Self {
                bytes: bytes.to_vec(),
                cursor: 0,
            }
        }
    }

    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            let mut bytes = [0u8; 4];
            self.fill_bytes(&mut bytes);
            u32::from_le_bytes(bytes)
        }

        fn next_u64(&mut self) -> u64 {
            let mut bytes = [0u8; 8];
            self.fill_bytes(&mut bytes);
            u64::from_le_bytes(bytes)
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest {
                *byte = self
                    .bytes
                    .get(self.cursor)
                    .copied()
                    .unwrap_or(self.cursor as u8);
                self.cursor += 1;
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    impl CryptoRng for TestRng {}

    fn exchange(code: &str, expected_code: &str) -> Result<SessionSecret, ProtocolError> {
        let (sender, request) = SenderExchange::start(code, 4096, &mut TestRng::new(&[1; 128]))?;
        let (attempt, response) =
            receiver_response(expected_code, &request, 4096, &mut TestRng::new(&[2; 128]))?;
        let secret = sender.finish(code, &response)?;
        let confirm = secret.seal_confirm(&mut TestRng::new(&[3; 12]))?;
        attempt.verify_confirm(&confirm)?;
        Ok(secret)
    }

    #[test]
    fn matching_codes_establish_the_same_secret() {
        let secret = exchange("123456", "123456").expect("matching codes");
        let ciphertext = secret
            .seal_data(b"settings", &mut TestRng::new(&[4; 12]))
            .expect("seal");
        assert_ne!(&ciphertext[12..12 + b"settings".len()], b"settings");
    }

    #[test]
    fn wrong_code_fails_at_confirmation() {
        let (sender, request) =
            SenderExchange::start("123456", 4096, &mut TestRng::new(&[1; 128])).expect("sender");
        let (attempt, response) =
            receiver_response("000000", &request, 4096, &mut TestRng::new(&[2; 128]))
                .expect("receiver");
        let secret = sender.finish("123456", &response).expect("sender finish");
        let confirm = secret
            .seal_confirm(&mut TestRng::new(&[3; 12]))
            .expect("confirm");
        assert_eq!(
            attempt.verify_confirm(&confirm).err(),
            Some(ProtocolError::AuthenticationFailed)
        );
    }

    #[test]
    fn malformed_packets_and_invalid_codes_are_rejected() {
        assert_eq!(
            SenderExchange::start("12345", 4096, &mut TestRng::new(&[1; 128])).err(),
            Some(ProtocolError::InvalidCode)
        );
        assert_eq!(
            receiver_response("123456", &[0; 47], 4096, &mut TestRng::new(&[2; 128])).err(),
            Some(ProtocolError::InvalidFrame)
        );
        assert!(!is_valid_code("１２３４５６"));
        let secret = exchange("123456", "123456").expect("exchange");
        let (_, request) =
            SenderExchange::start("123456", 4096, &mut TestRng::new(&[6; 128])).expect("sender");
        assert!(secret.open_confirm(&request[..12]).is_err());
    }

    #[test]
    fn zero_max_payload_is_rejected_before_cpace_starts() {
        assert_eq!(
            SenderExchange::start("123456", 0, &mut TestRng::new(&[1; 128])).err(),
            Some(ProtocolError::PayloadTooLarge)
        );
    }

    #[test]
    fn generate_code_rejects_out_of_range_words() {
        let mut bytes = u32::MAX.to_le_bytes().to_vec();
        bytes.extend_from_slice(&123456u32.to_le_bytes());
        let code = generate_code(&mut TestRng::new(&bytes)).expect("code");
        assert_eq!(&*code, "123456");
    }

    #[test]
    fn generated_codes_are_six_ascii_digits() {
        for seed in 0..16 {
            let code = generate_code(&mut TestRng::new(&[seed; 4])).expect("code");
            assert!(is_valid_code(&code));
        }
    }
}
