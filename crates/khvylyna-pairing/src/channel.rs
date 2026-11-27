//! One-message directional encryption keys derived from an authenticated session.

use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    hkdf,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

const SENDER_TO_RECEIVER: &[u8] = b"khvylyna-settings-v1 sender-to-receiver";
const RECEIVER_TO_SENDER: &[u8] = b"khvylyna-settings-v1 receiver-to-sender";
const MAX_MESSAGE_BYTES: usize = 256 * 1024;
const NONCE_LEN: usize = 12;

/// Role of the device in a transfer.
#[derive(Clone, Copy)]
pub enum Role {
    /// The device sending settings.
    Sender,
    /// The device receiving settings.
    Receiver,
}

/// A failure to derive a key, encrypt, or authenticate a message.
#[derive(Debug)]
pub struct ChannelError;

impl std::fmt::Display for ChannelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("The protected settings transfer failed")
    }
}

impl std::error::Error for ChannelError {}

/// An encryption key that can be used for only one message.
pub struct SealingKey(Zeroizing<[u8; 32]>, &'static [u8]);

/// A decryption key that can be used for only one message.
pub struct OpeningKey(Zeroizing<[u8; 32]>, &'static [u8]);

/// Derives separate send and receive keys from an authenticated OPAQUE session.
pub fn derive_keys(
    session_key: &[u8],
    role: Role,
) -> Result<(SealingKey, OpeningKey), ChannelError> {
    if session_key.len() < 32 {
        return Err(ChannelError);
    }
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"khvylyna-settings-transfer-v1");
    let prk = salt.extract(session_key);
    let derive = |label: &'static [u8]| -> Result<Zeroizing<[u8; 32]>, ChannelError> {
        let mut key = Zeroizing::new([0u8; 32]);
        prk.expand(&[label], hkdf::HKDF_SHA256)
            .map_err(|_| ChannelError)?
            .fill(key.as_mut())
            .map_err(|_| ChannelError)?;
        Ok(key)
    };
    let (outgoing, incoming) = match role {
        Role::Sender => (SENDER_TO_RECEIVER, RECEIVER_TO_SENDER),
        Role::Receiver => (RECEIVER_TO_SENDER, SENDER_TO_RECEIVER),
    };
    Ok((
        SealingKey(derive(outgoing)?, outgoing),
        OpeningKey(derive(incoming)?, incoming),
    ))
}

impl SealingKey {
    /// Encrypts and authenticates one message, prefixing it with a random nonce.
    pub fn seal(self, plaintext: &[u8]) -> Result<Vec<u8>, ChannelError> {
        if plaintext.len() > MAX_MESSAGE_BYTES {
            return Err(ChannelError);
        }
        let mut nonce = [0u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| ChannelError)?;
        let key = LessSafeKey::new(
            UnboundKey::new(&aead::AES_256_GCM, self.0.as_ref()).map_err(|_| ChannelError)?,
        );
        let mut ciphertext = plaintext.to_vec();
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(self.1),
            &mut ciphertext,
        )
        .map_err(|_| ChannelError)?;
        let mut message = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        message.extend_from_slice(&nonce);
        message.extend_from_slice(&ciphertext);
        Ok(message)
    }
}

impl OpeningKey {
    /// Authenticates and decrypts one message.
    pub fn open(self, message: &[u8]) -> Result<Zeroizing<Vec<u8>>, ChannelError> {
        if message.len() < NONCE_LEN + aead::AES_256_GCM.tag_len()
            || message.len() > NONCE_LEN + MAX_MESSAGE_BYTES + aead::AES_256_GCM.tag_len()
        {
            return Err(ChannelError);
        }
        let nonce: [u8; NONCE_LEN] = message[..NONCE_LEN].try_into().map_err(|_| ChannelError)?;
        let key = LessSafeKey::new(
            UnboundKey::new(&aead::AES_256_GCM, self.0.as_ref()).map_err(|_| ChannelError)?,
        );
        let mut plaintext = Zeroizing::new(message[NONCE_LEN..].to_vec());
        let length = key
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.1),
                &mut plaintext,
            )
            .map_err(|_| ChannelError)?
            .len();
        plaintext.truncate(length);
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_directions_round_trip_with_separate_keys() {
        let (sender_seal, sender_open) = derive_keys(&[7; 64], Role::Sender).expect("sender");
        let (receiver_seal, receiver_open) =
            derive_keys(&[7; 64], Role::Receiver).expect("receiver");
        let offer = sender_seal.seal(b"settings").expect("encrypt offer");
        assert_eq!(
            &**receiver_open.open(&offer).expect("decrypt offer"),
            b"settings"
        );
        let response = receiver_seal.seal(b"accepted").expect("encrypt response");
        assert_eq!(
            &**sender_open.open(&response).expect("decrypt response"),
            b"accepted"
        );
    }

    #[test]
    fn tampering_and_wrong_direction_are_rejected() {
        let (sender_seal, sender_open) = derive_keys(&[8; 64], Role::Sender).expect("sender");
        let (_, receiver_open) = derive_keys(&[8; 64], Role::Receiver).expect("receiver");
        let mut message = sender_seal.seal(b"settings").expect("encrypt");
        assert!(sender_open.open(&message).is_err());
        message[NONCE_LEN] ^= 1;
        assert!(receiver_open.open(&message).is_err());
    }

    #[test]
    fn malformed_or_oversized_messages_are_rejected() {
        let (_, opening) = derive_keys(&[9; 64], Role::Receiver).expect("receiver");
        assert!(opening.open(&[0; 12]).is_err());
        let (sealing, _) = derive_keys(&[9; 64], Role::Sender).expect("sender");
        assert!(sealing.seal(&vec![0; MAX_MESSAGE_BYTES + 1]).is_err());
        assert!(derive_keys(&[0; 16], Role::Sender).is_err());
    }
}
