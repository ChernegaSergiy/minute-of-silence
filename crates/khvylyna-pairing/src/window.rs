//! Time-limited, single-use receiver pairing window.

use std::time::{Duration, Instant};
use zeroize::Zeroizing;

use crate::{PairingError, Receiver, ReceiverExchange, generate_code};

const MAX_ATTEMPTS: usize = 5;
const CODE_LIFETIME: Duration = Duration::from_secs(120);

/// A code and its receiver state, valid for a limited time and number of attempts.
pub struct PairingWindow {
    code: Zeroizing<String>,
    receiver: Receiver,
    expires_at: Instant,
    attempts: usize,
    consumed: bool,
}

impl PairingWindow {
    /// Creates a fresh code that expires two minutes after the supplied time.
    pub fn new(now: Instant) -> Result<Self, PairingError> {
        let code = generate_code()?;
        let receiver = Receiver::new(&code)?;
        Ok(Self {
            code,
            receiver,
            expires_at: now + CODE_LIFETIME,
            attempts: 0,
            consumed: false,
        })
    }

    /// Returns the code only while the window can accept another attempt.
    pub fn code(&self, now: Instant) -> Option<&str> {
        if self.available(now) {
            Some(&self.code)
        } else {
            None
        }
    }

    /// Returns the remaining code lifetime while pairing is available.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.code(now)?;
        Some(self.expires_at.saturating_duration_since(now))
    }

    /// Consumes one attempt and begins authenticating a sender.
    pub fn start(
        &mut self,
        request: &[u8],
        now: Instant,
    ) -> Result<(ReceiverExchange, Vec<u8>), PairingError> {
        if !self.available(now) {
            return Err(PairingError::WindowUnavailable);
        }
        self.attempts += 1;
        self.receiver.start(request)
    }

    /// Verifies the sender and consumes the code after the first successful exchange.
    pub fn finish(
        &mut self,
        exchange: ReceiverExchange,
        finalization: &[u8],
        now: Instant,
    ) -> Result<Zeroizing<Vec<u8>>, PairingError> {
        if self.consumed || now >= self.expires_at {
            return Err(PairingError::WindowUnavailable);
        }
        let key = exchange.finish(finalization)?;
        self.consumed = true;
        Ok(key)
    }

    fn available(&self, now: Instant) -> bool {
        !self.consumed && now < self.expires_at && self.attempts < MAX_ATTEMPTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SenderExchange;

    #[test]
    fn code_expires_at_the_deadline() {
        let now = Instant::now();
        let window = PairingWindow::new(now).expect("window");
        assert!(
            window
                .code(now + CODE_LIFETIME - Duration::from_nanos(1))
                .is_some()
        );
        assert!(window.code(now + CODE_LIFETIME).is_none());
        assert_eq!(window.remaining(now), Some(CODE_LIFETIME));
        assert_eq!(window.remaining(now + CODE_LIFETIME), None);
    }

    #[test]
    fn failed_requests_are_limited() {
        let now = Instant::now();
        let mut window = PairingWindow::new(now).expect("window");
        for _ in 0..MAX_ATTEMPTS {
            assert!(window.start(&[], now).is_err());
        }
        assert!(window.code(now).is_none());
        assert!(matches!(
            window.start(&[], now),
            Err(PairingError::WindowUnavailable)
        ));
    }

    #[test]
    fn only_one_successful_exchange_can_consume_the_code() {
        let now = Instant::now();
        let mut window = PairingWindow::new(now).expect("window");
        let code = window.code(now).expect("code").to_string();
        let (first_sender, first_request) = SenderExchange::start(&code).expect("sender");
        let (second_sender, second_request) = SenderExchange::start(&code).expect("sender");
        let (first_receiver, first_response) = window.start(&first_request, now).expect("receiver");
        let (second_receiver, second_response) =
            window.start(&second_request, now).expect("receiver");
        let (first_confirmation, _) = first_sender.finish(&code, &first_response).expect("sender");
        let (second_confirmation, _) = second_sender
            .finish(&code, &second_response)
            .expect("sender");
        assert!(
            window
                .finish(first_receiver, &first_confirmation, now)
                .is_ok()
        );
        assert!(window.code(now).is_none());
        assert!(matches!(
            window.finish(second_receiver, &second_confirmation, now),
            Err(PairingError::WindowUnavailable)
        ));
    }
}
