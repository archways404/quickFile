// src/encode.rs

use base64::encode;

/// Encodes the given file content to a Base64 string.
///
/// # Arguments
///
/// * `file_content` - A vector of bytes representing the file content.
///
/// # Returns
///
/// * A String containing the Base64 encoded representation of the file content.
pub fn encode_to_base64(file_content: &[u8]) -> String {
    encode(file_content)
}
