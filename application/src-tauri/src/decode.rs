// src/decode.rs

use base64::decode;
use std::error::Error;

/// Decodes a Base64 encoded string to bytes.
///
/// # Arguments
///
/// * `encoded_data` - A string containing the Base64 encoded data.
///
/// # Returns
///
/// * A vector of bytes containing the decoded data.
pub fn decode_from_base64(encoded_data: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let decoded_data = decode(encoded_data)?;
    Ok(decoded_data)
}
