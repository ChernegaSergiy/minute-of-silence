//! Short-lived, single-use receiver pairing window with online-attempt limits.

use alloc::{string::String, vec::Vec};
use rand_core::{CryptoRng, RngCore};
use zeroize::Zeroizing;

use crate::{
    ProtocolError,
    pake::{ConfirmedAttempt, PairingAttempt, generate_code, receiver_response},
};

/// Maximum number of PAKE attempts accepted for one code.
pub const MAX_ATTEMPTS: usize = 5;
/// Code lifetime in milliseconds.
pub const CODE_LIFETIME_MS: u64 = 120_000;

/// A displayed code and the state needed to authenticate one sender.
pub struct PairingWindow {
    code: Zeroizing<String>,
    expires_at_ms: u64,
    attempts: usize,
    consumed: bool,
}

impl PairingWindow {
    /// Creates a fresh code that expires two minutes after `now_ms`.
    pub fn new<R: RngCore + CryptoRng>(now_ms: u64, rng: &mut R) -> Result<Self, ProtocolError> {
        let code = generate_code(rng)?;
        Ok(Self {
            code,
            expires_at_ms: now_ms.saturating_add(CODE_LIFETIME_MS),
            attempts: 0,
            consumed: false,
        })
    }

    /// Returns the code only while the window can accept another attempt.
    pub fn code(&self, now_ms: u64) -> Option<&str> {
        if self.available(now_ms) {
            Some(&self.code)
        } else {
            None
        }
    }

    /// Returns the remaining code lifetime in milliseconds.
    pub fn remaining(&self, now_ms: u64) -> Option<u64> {
        self.code(now_ms)?;
        Some(self.expires_at_ms.saturating_sub(now_ms))
    }

    /// Consumes one attempt and responds to the sender's CPace packet.
    pub fn start<R: RngCore + CryptoRng>(
        &mut self,
        request: &[u8],
        now_ms: u64,
        max_payload: u16,
        rng: &mut R,
    ) -> Result<(PairingAttempt, Vec<u8>), ProtocolError> {
        if !self.available(now_ms) {
            return Err(ProtocolError::WindowUnavailable);
        }
        self.attempts += 1;
        receiver_response(&self.code, request, max_payload, rng)
    }

    /// Consumes the code after the sender proved knowledge of it.
    pub fn finish(
        &mut self,
        attempt: ConfirmedAttempt,
        now_ms: u64,
    ) -> Result<crate::channel::SessionSecret, ProtocolError> {
        if self.consumed || now_ms >= self.expires_at_ms {
            return Err(ProtocolError::WindowUnavailable);
        }
        self.consumed = true;
        Ok(attempt.into_secret())
    }

    fn available(&self, now_ms: u64) -> bool {
        !self.consumed && now_ms < self.expires_at_ms && self.attempts < MAX_ATTEMPTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pake::SenderExchange;
    use alloc::string::ToString;
    use rand_core::{CryptoRng, RngCore};

    struct TestRng(u8);

    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            u32::from_le_bytes([self.0, self.0, self.0, self.0])
        }

        fn next_u64(&mut self) -> u64 {
            u64::from_le_bytes([self.0; 8])
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest {
                *byte = self.0;
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    impl CryptoRng for TestRng {}

    fn window() -> PairingWindow {
        PairingWindow::new(0, &mut TestRng(7)).expect("window")
    }

    #[test]
    fn code_expires_at_the_deadline() {
        let window = window();
        assert!(window.code(CODE_LIFETIME_MS - 1).is_some());
        assert!(window.code(CODE_LIFETIME_MS).is_none());
        assert_eq!(window.remaining(0), Some(CODE_LIFETIME_MS));
        assert_eq!(window.remaining(CODE_LIFETIME_MS), None);
    }

    #[test]
    fn failed_requests_are_limited() {
        let mut window = window();
        for _ in 0..MAX_ATTEMPTS {
            assert!(window.start(&[], 0, 4096, &mut TestRng(1)).is_err());
        }
        assert!(window.code(0).is_none());
        assert_eq!(
            window.start(&[], 0, 4096, &mut TestRng(1)).err(),
            Some(ProtocolError::WindowUnavailable)
        );
    }

    #[test]
    fn only_one_successful_exchange_can_consume_the_code() {
        let mut window = window();
        let code = window.code(0).expect("code").to_string();
        let (first_sender, first_request) =
            SenderExchange::start(&code, 4096, &mut TestRng(1)).expect("sender");
        let (second_sender, second_request) =
            SenderExchange::start(&code, 4096, &mut TestRng(2)).expect("sender");
        let (first_attempt, first_response) = window
            .start(&first_request, 0, 4096, &mut TestRng(3))
            .expect("first attempt");
        let (second_attempt, second_response) = window
            .start(&second_request, 0, 4096, &mut TestRng(4))
            .expect("second attempt");
        let first_secret = first_sender.finish(&code, &first_response).expect("finish");
        let second_secret = second_sender
            .finish(&code, &second_response)
            .expect("finish");
        let first_confirmed = first_attempt
            .verify_confirm(&first_secret.seal_confirm(&mut TestRng(5)).expect("confirm"))
            .expect("verified");
        let second_confirmed = second_attempt
            .verify_confirm(
                &second_secret
                    .seal_confirm(&mut TestRng(6))
                    .expect("confirm"),
            )
            .expect("verified");

        assert!(window.finish(first_confirmed, 0).is_ok());
        assert!(window.code(0).is_none());
        assert_eq!(
            window.finish(second_confirmed, 0).err(),
            Some(ProtocolError::WindowUnavailable)
        );
    }
}
