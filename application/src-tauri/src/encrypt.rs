// src/encrypt.rs

use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Pkcs7;
use hex_literal::hex;
use rand::{thread_rng, Rng};
use rand::distributions::Standard;
use rand::RngCore;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

const KEY: [u8; 32] = hex!("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");

pub fn encrypt(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Generate a random initialization vector (IV)
    let mut iv = [0u8; 16];
    thread_rng().fill_bytes(&mut iv);

    // Create an AES-256-CBC cipher instance
    let cipher = Aes256Cbc::new_from_slices(&KEY, &iv)?;

    // Encrypt the data
    let ciphertext = cipher.encrypt_vec(data);

    // Prepend IV to the ciphertext for use in decryption
    let mut encrypted_data = iv.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);

    Ok(encrypted_data)
}
