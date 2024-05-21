#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{command, Builder, generate_context, generate_handler};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

// Include the encode, decode, encrypt, decrypt, and chunks modules
mod encode;
mod decode;
mod encrypt;
mod decrypt;
mod chunks;

use encode::encode_to_base64;
use decode::decode_from_base64;
use encrypt::encrypt;
use decrypt::decrypt;
use chunks::split_into_chunks;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1MB

#[command]
fn process_file(file_name: String, file_content: Vec<u8>) -> Result<String, String> {
    // Print the file name and size
    println!("Received file: {}", file_name);
    println!("File size: {} bytes", file_content.len());

    // Encode file content to Base64
    let encoded_content = encode_to_base64(&file_content);
    println!("Base64 Encoded Content: {}", encoded_content);

    // Decode the Base64 content back to bytes
    let decoded_content = decode_from_base64(&encoded_content)
        .map_err(|e| format!("Base64 decoding failed: {}", e.to_string()))?;
    println!("Decoded Content matches original: {}", decoded_content == file_content);

    // Encrypt the file content
    let encrypted_content = encrypt(&file_content)
        .map_err(|e| format!("Encryption failed: {}", e.to_string()))?;
    println!("Encrypted Content: {:?}", encrypted_content);

    // Split the encrypted content into 1MB chunks
    let encrypted_chunks = split_into_chunks(&encrypted_content, CHUNK_SIZE);
    println!("Encrypted Content divided into {} chunks", encrypted_chunks.len());

    // Save each chunk to the local file system
    for (i, chunk) in encrypted_chunks.iter().enumerate() {
        let chunk_path = PathBuf::from(format!("./{}_chunk_{}.enc", file_name, i));
        let mut chunk_file = File::create(&chunk_path).map_err(|e| e.to_string())?;
        chunk_file.write_all(&chunk).map_err(|e| e.to_string())?;
        println!("Saved chunk {} successfully", i);
    }

    // Decrypt the file content for testing purposes
    let decrypted_content = decrypt(&encrypted_content)
        .map_err(|e| format!("Decryption failed: {}", e.to_string()))?;
    println!("Decrypted Content matches original: {}", decrypted_content == file_content);

    Ok("File processed, encoded, decoded, encrypted, divided into chunks, and decrypted successfully".into())
}

fn main() {
    Builder::default()
        .invoke_handler(generate_handler![process_file])
        .run(generate_context!())
        .expect("error while running tauri application");
}
