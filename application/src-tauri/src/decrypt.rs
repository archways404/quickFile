// src/decrypt.rs

use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use hex_literal::hex;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const KEY: [u8; 32] = hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");

pub fn decrypt(encrypted_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Extract the IV from the beginning of the encrypted data
    let (iv, ciphertext) = encrypted_data.split_at(16);

    // Create an AES-256-CBC cipher instance
    let cipher = Aes256Cbc::new_from_slices(&KEY, iv)?;

    // Decrypt the data
    let decrypted_data = cipher.decrypt_vec(ciphertext)?;

    Ok(decrypted_data)
}
