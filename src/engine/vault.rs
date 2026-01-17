use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use crate::{Result, Error};

/// Encrypts a plaintext string using AES-256-GCM and a 32-byte key.
/// 
/// Returns a hex-encoded string containing the nonce followed by the ciphertext.
pub fn encrypt(plaintext: &str, key: &[u8]) -> Result<String> {
    if key.len() != 32 {
        return Err(Error::Internal("Key must be 32 bytes".to_string()));
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| Error::Internal(e.to_string()))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96 bits / 12 bytes
    let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|e| Error::Internal(e.to_string()))?;

    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(hex::encode(combined))
}

/// Decrypts a hex-encoded ciphertext string using AES-256-GCM and a 32-byte key.
/// 
/// The `cipher_hex` must be the output of [`encrypt`], containing the 12-byte
/// nonce followed by the ciphertext.
pub fn decrypt(cipher_hex: &str, key: &[u8]) -> Result<String> {
    if key.len() != 32 {
        return Err(Error::Internal("Key must be 32 bytes".to_string()));
    }
    let combined = hex::decode(cipher_hex).map_err(|e| Error::Internal(e.to_string()))?;
    if combined.len() < 12 {
        return Err(Error::Internal("Ciphertext too short".to_string()));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| Error::Internal(e.to_string()))?;
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher.decrypt(nonce, ciphertext).map_err(|_| Error::Internal("decryption failed (wrong key or tampered data)".to_string()))?;
    String::from_utf8(plaintext_bytes).map_err(|e| Error::Internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = b"thisis32byteslongsecretkey123456";
        let plaintext = "Hello, Celerix!";
        let ciphertext = encrypt(plaintext, key).unwrap();
        assert_ne!(ciphertext, plaintext);
        let decrypted = decrypt(&ciphertext, key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let key1 = b"thisis32byteslongsecretkey123456";
        let key2 = b"another32byteslongsecretkey65432";
        let plaintext = "Secret message";
        let ciphertext = encrypt(plaintext, key1).unwrap();
        assert!(decrypt(&ciphertext, key2).is_err());
    }
}
