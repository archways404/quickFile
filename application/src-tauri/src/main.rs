#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{command, Builder, generate_context, generate_handler};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::runtime::Runtime;

// Include the encode, decode, encrypt, decrypt, chunks, and upload modules
mod encode;
mod decode;
mod encrypt;
mod decrypt;
mod chunks;
mod upload;

use encode::encode_to_base64_temp;
use decode::decode_from_base64_temp;
use encrypt::encrypt_temp;
use decrypt::decrypt_temp;
use chunks::split_into_temp_files;
use upload::upload_chunks;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1MB

#[command]
fn process_file(file_name: String, file_content: Vec<u8>) -> Result<String, String> {
    // Print the file name and size
    println!("Received file: {}", file_name);
    println!("File size: {} bytes", file_content.len());

    // Encode file content to Base64 and write to a temporary file
    let (encoded_temp_file, encoded_temp_path) = encode_to_base64_temp(&file_content)
        .map_err(|e| format!("Base64 encoding failed: {}", e.to_string()))?;
    println!("Base64 Encoded Content written to temp file");

    // Decode the Base64 content from the temporary file
    let (decoded_temp_file, decoded_temp_path) = decode_from_base64_temp(&encoded_temp_file)
        .map_err(|e| format!("Base64 decoding failed: {}", e.to_string()))?;
    println!("Decoded Content written to temp file");

    // Encrypt the file content and write to a temporary file
    let (encrypted_temp_file, encrypted_temp_path) = encrypt_temp(&file_content)
        .map_err(|e| format!("Encryption failed: {}", e.to_string()))?;
    println!("Encrypted Content written to temp file");

    // Split the encrypted content into 1MB chunks and write to temporary files
    let mut encrypted_content = Vec::new();
    let mut encrypted_temp_handle = encrypted_temp_file.reopen().map_err(|e| e.to_string())?;
    encrypted_temp_handle.read_to_end(&mut encrypted_content).map_err(|e| e.to_string())?;
    
    let temp_files = split_into_temp_files(&encrypted_content, CHUNK_SIZE)
        .map_err(|e| format!("Splitting into temp files failed: {}", e.to_string()))?;
    println!("Encrypted Content divided into {} chunks", temp_files.len());

    // Use tokio runtime to run the async upload function
    let rt = Runtime::new().unwrap();
    rt.block_on(upload_chunks(temp_files)).map_err(|e| format!("Upload failed: {}", e.to_string()))?;

    // Decrypt the file content from the temporary file for testing purposes
    let (decrypted_temp_file, decrypted_temp_path) = decrypt_temp(&encrypted_temp_file)
        .map_err(|e| format!("Decryption failed: {}", e.to_string()))?;
    println!("Decrypted Content written to temp file");

    // Verify the decrypted content
    let mut decrypted_content = Vec::new();
    let mut decrypted_temp_handle = decrypted_temp_file.reopen().map_err(|e| e.to_string())?;
    decrypted_temp_handle.read_to_end(&mut decrypted_content).map_err(|e| e.to_string())?;
    println!("Decrypted Content matches original: {}", decrypted_content == file_content);

    Ok("File processed, encoded, decoded, encrypted, divided into chunks, uploaded, and decrypted successfully".into())
}

fn main() {
    Builder::default()
        .invoke_handler(generate_handler![process_file])
        .run(generate_context!())
        .expect("error while running tauri application");
}
