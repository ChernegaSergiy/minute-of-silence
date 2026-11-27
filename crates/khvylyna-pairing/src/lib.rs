//! Password-authenticated key exchange for a single settings-transfer session.

pub mod channel;
pub mod window;

use opaque_ke::{
    CipherSuite, ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, ServerLogin, ServerLoginStartParameters, ServerRegistration, ServerSetup,
    key_exchange::tripledh::TripleDh,
};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

const IDENTITY: &[u8] = b"khvylyna-settings-transfer-v1";

struct Suite;

impl CipherSuite for Suite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeGroup = opaque_ke::Ristretto255;
    type KeyExchange = TripleDh;
    type Ksf = argon2::Argon2<'static>;
}

/// An invalid code or failed authenticated exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingError {
    /// The code must consist of six ASCII digits.
    InvalidCode,
    /// The exchange failed or contained an invalid message.
    ExchangeFailed,
    /// The code expired, was already used, or has no attempts left.
    WindowUnavailable,
}

impl std::fmt::Display for PairingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCode => formatter.write_str("The pairing code must contain six digits"),
            Self::ExchangeFailed => formatter.write_str("The authenticated exchange failed"),
            Self::WindowUnavailable => {
                formatter.write_str("The pairing code is no longer available")
            }
        }
    }
}

impl std::error::Error for PairingError {}

fn validate_code(code: &str) -> Result<(), PairingError> {
    if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(PairingError::InvalidCode)
    }
}

/// Generates an unbiased six-digit code using the operating system's secure RNG.
pub fn generate_code() -> Result<Zeroizing<String>, PairingError> {
    const CODE_SPACE: u32 = 1_000_000;
    const LIMIT: u32 = u32::MAX - (u32::MAX % CODE_SPACE);
    let mut rng = OsRng;
    loop {
        let mut bytes = [0u8; 4];
        rng.try_fill_bytes(&mut bytes)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let value = u32::from_le_bytes(bytes);
        if value < LIMIT {
            return Ok(Zeroizing::new(format!("{:06}", value % CODE_SPACE)));
        }
    }
}

/// Short-lived receiver credentials for one pairing code.
pub struct Receiver {
    setup: ServerSetup<Suite>,
    registration: Zeroizing<Vec<u8>>,
}

impl Receiver {
    /// Registers a code locally without transmitting it.
    pub fn new(code: &str) -> Result<Self, PairingError> {
        validate_code(code)?;
        let mut rng = OsRng;
        let setup = ServerSetup::<Suite>::new(&mut rng);
        let client = ClientRegistration::<Suite>::start(&mut rng, code.as_bytes())
            .map_err(|_| PairingError::ExchangeFailed)?;
        let response = ServerRegistration::<Suite>::start(&setup, client.message, IDENTITY)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let upload = client
            .state
            .finish(
                &mut rng,
                code.as_bytes(),
                response.message,
                ClientRegistrationFinishParameters::default(),
            )
            .map_err(|_| PairingError::ExchangeFailed)?;
        let registration = ServerRegistration::<Suite>::finish(upload.message);
        Ok(Self {
            setup,
            registration: Zeroizing::new(registration.serialize().to_vec()),
        })
    }

    /// Begins authenticating a sender's credential request.
    pub fn start(&self, request: &[u8]) -> Result<(ReceiverExchange, Vec<u8>), PairingError> {
        let registration = ServerRegistration::<Suite>::deserialize(&self.registration)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let request = CredentialRequest::<Suite>::deserialize(request)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let result = ServerLogin::start(
            &mut OsRng,
            &self.setup,
            Some(registration),
            request,
            IDENTITY,
            ServerLoginStartParameters::default(),
        )
        .map_err(|_| PairingError::ExchangeFailed)?;
        Ok((
            ReceiverExchange(result.state),
            result.message.serialize().to_vec(),
        ))
    }
}

/// Receiver state awaiting the sender's final confirmation.
pub struct ReceiverExchange(ServerLogin<Suite>);

impl ReceiverExchange {
    /// Verifies the sender and returns the shared session key.
    pub fn finish(self, finalization: &[u8]) -> Result<Zeroizing<Vec<u8>>, PairingError> {
        let message = CredentialFinalization::<Suite>::deserialize(finalization)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let result = self
            .0
            .finish(message)
            .map_err(|_| PairingError::ExchangeFailed)?;
        Ok(Zeroizing::new(result.session_key.to_vec()))
    }
}

/// Sender state awaiting the receiver's authenticated response.
pub struct SenderExchange(ClientLogin<Suite>);

impl SenderExchange {
    /// Starts a sender exchange using the code shown on the receiving device.
    pub fn start(code: &str) -> Result<(Self, Vec<u8>), PairingError> {
        validate_code(code)?;
        let result = ClientLogin::<Suite>::start(&mut OsRng, code.as_bytes())
            .map_err(|_| PairingError::ExchangeFailed)?;
        Ok((Self(result.state), result.message.serialize().to_vec()))
    }

    /// Verifies the receiver and returns the final confirmation and session key.
    pub fn finish(
        self,
        code: &str,
        response: &[u8],
    ) -> Result<(Vec<u8>, Zeroizing<Vec<u8>>), PairingError> {
        validate_code(code)?;
        let response = CredentialResponse::<Suite>::deserialize(response)
            .map_err(|_| PairingError::ExchangeFailed)?;
        let result = self
            .0
            .finish(
                code.as_bytes(),
                response,
                ClientLoginFinishParameters::default(),
            )
            .map_err(|_| PairingError::ExchangeFailed)?;
        Ok((
            result.message.serialize().to_vec(),
            Zeroizing::new(result.session_key.to_vec()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_codes_produce_the_same_session_key() {
        let receiver = Receiver::new("123456").expect("receiver");
        let (sender, request) = SenderExchange::start("123456").expect("sender");
        let (receiver_exchange, response) = receiver.start(&request).expect("receiver exchange");
        let (confirmation, sender_key) = sender.finish("123456", &response).expect("sender finish");
        let receiver_key = receiver_exchange
            .finish(&confirmation)
            .expect("receiver finish");
        assert_eq!(&*sender_key, &*receiver_key);
    }

    #[test]
    fn incorrect_codes_cannot_authenticate_the_receiver() {
        let receiver = Receiver::new("123456").expect("receiver");
        let (sender, request) = SenderExchange::start("123457").expect("sender");
        let (_, response) = receiver.start(&request).expect("receiver exchange");
        assert!(matches!(
            sender.finish("123457", &response),
            Err(PairingError::ExchangeFailed)
        ));
    }

    #[test]
    fn altered_or_malformed_messages_are_rejected() {
        let receiver = Receiver::new("123456").expect("receiver");
        assert!(matches!(
            receiver.start(&[]),
            Err(PairingError::ExchangeFailed)
        ));
        let (sender, request) = SenderExchange::start("123456").expect("sender");
        let (receiver_exchange, response) = receiver.start(&request).expect("receiver exchange");
        let (mut confirmation, _) = sender.finish("123456", &response).expect("sender finish");
        confirmation[0] ^= 1;
        assert!(matches!(
            receiver_exchange.finish(&confirmation),
            Err(PairingError::ExchangeFailed)
        ));
    }

    #[test]
    fn pairing_codes_must_be_six_ascii_digits() {
        for code in ["", "12345", "1234567", "12 456", "１２３４５６"] {
            assert!(matches!(
                Receiver::new(code),
                Err(PairingError::InvalidCode)
            ));
            assert!(matches!(
                SenderExchange::start(code),
                Err(PairingError::InvalidCode)
            ));
        }
    }

    #[test]
    fn generated_codes_have_six_ascii_digits() {
        for _ in 0..100 {
            let code = generate_code().expect("secure random code");
            assert_eq!(validate_code(&code), Ok(()));
        }
    }
}
