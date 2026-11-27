//! HKDF-SHA256 key schedule and one-shot AES-256-GCM messages.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{AeadInOut, KeyInit},
};
use alloc::vec::Vec;
use hkdf::Hkdf;
use rand_core::{CryptoRng, RngCore};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::ProtocolError;

/// HKDF extract salt used for every direction label.
pub const HKDF_SALT: &[u8] = b"khvylyna-pairing-v1 HKDF-SHA256 salt";
/// Key label and AEAD associated data for the sender's code confirmation.
pub const AAD_CONFIRM: &[u8] = b"khvylyna-pairing-v1 sender-to-receiver confirm";
/// Key label and AEAD associated data for the sender's settings payload.
pub const AAD_DATA: &[u8] = b"khvylyna-pairing-v1 sender-to-receiver data";
/// Key label and AEAD associated data for the receiver's result.
pub const AAD_ACK: &[u8] = b"khvylyna-pairing-v1 receiver-to-sender ack";
/// Plaintext authenticated by the one-shot confirmation message.
pub const CONFIRM_PLAINTEXT: &[u8] = b"khvylyna-pairing-v1 confirmed";
/// Accepted result byte in an encrypted acknowledgement.
pub const ACK_ACCEPTED: u8 = 1;
/// Rejected result byte in an encrypted acknowledgement.
pub const ACK_REJECTED: u8 = 0;

/// Random nonce prefix length in bytes.
pub const NONCE_LEN: usize = 12;
/// AES-GCM authentication tag length in bytes.
pub const TAG_LEN: usize = 16;

/// Returns the encoded size of an encrypted message of `plaintext_len` bytes.
pub const fn encrypted_len(plaintext_len: usize) -> usize {
    NONCE_LEN + plaintext_len + TAG_LEN
}

/// Session secret derived from the two CPace keys.
pub struct SessionSecret(Zeroizing<[u8; 64]>);

impl SessionSecret {
    /// Combines the two 32-byte CPace keys into a session secret.
    pub fn new(k1: [u8; 32], k2: [u8; 32]) -> Self {
        let mut secret = Zeroizing::new([0u8; 64]);
        secret[..32].copy_from_slice(&k1);
        secret[32..].copy_from_slice(&k2);
        Self(secret)
    }

    /// Encrypts one confirmation message using a fresh random nonce.
    pub fn seal_confirm<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.seal(AAD_CONFIRM, CONFIRM_PLAINTEXT, rng)
    }

    /// Encrypts one settings payload using a fresh random nonce.
    pub fn seal_data<R: RngCore + CryptoRng>(
        &self,
        payload: &[u8],
        rng: &mut R,
    ) -> Result<Vec<u8>, ProtocolError> {
        self.seal(AAD_DATA, payload, rng)
    }

    /// Encrypts one accept/reject result using a fresh random nonce.
    pub fn seal_ack<R: RngCore + CryptoRng>(
        &self,
        accepted: bool,
        rng: &mut R,
    ) -> Result<Vec<u8>, ProtocolError> {
        let status = if accepted { ACK_ACCEPTED } else { ACK_REJECTED };
        self.seal(AAD_ACK, &[status], rng)
    }

    /// Verifies the sender's confirmation message.
    pub fn open_confirm(&self, message: &[u8]) -> Result<(), ProtocolError> {
        let plaintext = self.open(AAD_CONFIRM, message)?;
        if plaintext.as_slice() == CONFIRM_PLAINTEXT {
            Ok(())
        } else {
            Err(ProtocolError::AuthenticationFailed)
        }
    }

    /// Authenticates and decrypts the sender's settings payload.
    pub fn open_data(&self, message: &[u8]) -> Result<Zeroizing<Vec<u8>>, ProtocolError> {
        self.open(AAD_DATA, message)
    }

    /// Authenticates and decrypts the receiver's result.
    pub fn open_ack(&self, message: &[u8]) -> Result<bool, ProtocolError> {
        let plaintext = self.open(AAD_ACK, message)?;
        match plaintext.as_slice() {
            [ACK_ACCEPTED] => Ok(true),
            [ACK_REJECTED] => Ok(false),
            _ => Err(ProtocolError::InvalidFrame),
        }
    }

    fn seal<R: RngCore + CryptoRng>(
        &self,
        label: &[u8],
        plaintext: &[u8],
        rng: &mut R,
    ) -> Result<Vec<u8>, ProtocolError> {
        let key = self.derive_key(label)?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref())
            .map_err(|_| ProtocolError::ChannelFailed)?;
        let mut nonce = [0u8; NONCE_LEN];
        rng.try_fill_bytes(&mut nonce)
            .map_err(|_| ProtocolError::ChannelFailed)?;

        let mut ciphertext = Vec::with_capacity(encrypted_len(plaintext.len()));
        ciphertext.extend_from_slice(plaintext);
        cipher
            .encrypt_in_place(&Nonce::from(nonce), label, &mut ciphertext)
            .map_err(|_| ProtocolError::ChannelFailed)?;

        let mut message = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        message.extend_from_slice(&nonce);
        message.extend_from_slice(&ciphertext);
        Ok(message)
    }

    fn open(&self, label: &[u8], message: &[u8]) -> Result<Zeroizing<Vec<u8>>, ProtocolError> {
        if message.len() < encrypted_len(0) {
            return Err(ProtocolError::InvalidFrame);
        }
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&message[..NONCE_LEN]);
        let key = self.derive_key(label)?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref())
            .map_err(|_| ProtocolError::ChannelFailed)?;
        let mut plaintext = Zeroizing::new(message[NONCE_LEN..].to_vec());
        cipher
            .decrypt_in_place(&Nonce::from(nonce), label, &mut *plaintext)
            .map_err(|_| ProtocolError::AuthenticationFailed)?;
        Ok(plaintext)
    }

    fn derive_key(&self, label: &[u8]) -> Result<Zeroizing<[u8; 32]>, ProtocolError> {
        let hkdf = Hkdf::<Sha256>::new(Some(HKDF_SALT), self.0.as_ref());
        let mut key = Zeroizing::new([0u8; 32]);
        hkdf.expand(label, &mut key[..])
            .map_err(|_| ProtocolError::ChannelFailed)?;
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRng(u8);

    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            u32::from_le_bytes([
                self.0,
                self.0.wrapping_add(1),
                self.0.wrapping_add(2),
                self.0.wrapping_add(3),
            ])
        }

        fn next_u64(&mut self) -> u64 {
            u64::from_le_bytes([
                self.0,
                self.0.wrapping_add(1),
                self.0.wrapping_add(2),
                self.0.wrapping_add(3),
                self.0.wrapping_add(4),
                self.0.wrapping_add(5),
                self.0.wrapping_add(6),
                self.0.wrapping_add(7),
            ])
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest {
                *byte = self.0;
                self.0 = self.0.wrapping_add(1);
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    impl CryptoRng for TestRng {}

    #[test]
    fn each_direction_and_purpose_uses_a_distinct_one_shot_key() {
        let secret = SessionSecret::new([7; 32], [9; 32]);
        let confirm = secret.seal_confirm(&mut TestRng(1)).expect("confirm");
        let data = secret
            .seal_data(b"settings", &mut TestRng(2))
            .expect("data");
        let ack = secret.seal_ack(true, &mut TestRng(3)).expect("ack");

        secret.open_confirm(&confirm).expect("open confirm");
        assert_eq!(&*secret.open_data(&data).expect("open data"), b"settings");
        assert!(secret.open_ack(&ack).expect("open ack"));
        assert!(secret.open_data(&confirm).is_err());
        assert!(secret.open_confirm(&data).is_err());
    }

    #[test]
    fn tampered_and_truncated_messages_are_rejected() {
        let secret = SessionSecret::new([3; 32], [5; 32]);
        let mut message = secret
            .seal_data(b"private", &mut TestRng(11))
            .expect("seal");
        message[NONCE_LEN] ^= 1;
        assert!(secret.open_data(&message).is_err());
        assert!(secret.open_data(&[0u8; NONCE_LEN]).is_err());
    }
}
