#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{command, Builder, generate_context, generate_handler};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;

// Include the encode, decode, encrypt, decrypt, chunks, upload, and download modules
mod encode;
mod decode;
mod encrypt;
mod decrypt;
mod chunks;
mod upload;
mod download;

use encode::encode_to_base64_temp;
use decode::decode_from_base64_temp;
use encrypt::encrypt_temp;
use decrypt::decrypt_temp;
use chunks::split_into_temp_files;
use upload::upload_chunks;
use download::{download_files, rebuild_file, ResponseText};

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1MB

#[command]
async fn process_file(file_name: String, file_content: Vec<u8>) -> Result<String, String> {
    // Print the file name and size
    println!("Received file: {}", file_name);
    println!("File size: {} bytes", file_content.len());

    // Encode file content to Base64 and write to a temporary file
    let encoded_temp_file = encode_to_base64_temp(&file_content)
        .map_err(|e| format!("Base64 encoding failed: {}", e.to_string()))?;
    println!("Base64 Encoded Content written to temp file");

    // Decode the Base64 content from the temporary file
    let decoded_temp_file = decode_from_base64_temp(&encoded_temp_file)
        .map_err(|e| format!("Base64 decoding failed: {}", e.to_string()))?;
    println!("Decoded Content written to temp file");

    // Encrypt the file content and write to a temporary file
    let encrypted_temp_file = encrypt_temp(&file_content)
        .map_err(|e| format!("Encryption failed: {}", e.to_string()))?;
    println!("Encrypted Content written to temp file");

    // Split the encrypted content into 1MB chunks and write to temporary files
    let mut encrypted_content = Vec::new();
    let mut encrypted_temp_handle = encrypted_temp_file.reopen().map_err(|e| e.to_string())?;
    encrypted_temp_handle.read_to_end(&mut encrypted_content).map_err(|e| e.to_string())?;
    
    let temp_files = split_into_temp_files(&encrypted_content, CHUNK_SIZE)
        .map_err(|e| format!("Splitting into temp files failed: {}", e.to_string()))?;
    println!("Encrypted Content divided into {} chunks", temp_files.len());

    // Save each chunk to the local file system (optional)
    for (i, temp_file) in temp_files.iter().enumerate() {
        let chunk_path = PathBuf::from(format!("./chunks/chunk_{}.txt", i));
        let mut chunk_file = File::create(&chunk_path).map_err(|e| e.to_string())?;
        
        let mut temp_file_content = Vec::new();
        let mut temp_file_handle = temp_file.reopen().map_err(|e| e.to_string())?;
        temp_file_handle.read_to_end(&mut temp_file_content).map_err(|e| e.to_string())?;
        
        chunk_file.write_all(&temp_file_content).map_err(|e| e.to_string())?;
        println!("Saved chunk {} successfully", i);
    }

    // Upload the chunks
    upload_chunks(temp_files).await.map_err(|e| format!("Upload failed: {}", e.to_string()))?;
    println!("Chunks uploaded successfully");

    // Read the response_text.json file
    let file = File::open("response_text.json").map_err(|e| e.to_string())?;
    let response_text: ResponseText = serde_json::from_reader(file).map_err(|e| e.to_string())?;

    // Download the chunks
    let chunk_files = download_files(response_text.chunks).await.map_err(|e| format!("Download failed: {}", e.to_string()))?;
    println!("Chunks downloaded successfully");

    // Rebuild the file from the downloaded chunks
    let rebuilt_temp_file = rebuild_file(chunk_files).map_err(|e| format!("Rebuild failed: {}", e.to_string()))?;
    println!("Reconstructed file");

    // Save the reconstructed file with the original extension
    let extension = PathBuf::from(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("tmp")
        .to_string();

    let reconstructed_file_path = PathBuf::from(format!("reconstructed_file.{}", extension));
    let mut reconstructed_file = File::create(&reconstructed_file_path).map_err(|e| e.to_string())?;

    let mut rebuilt_content = Vec::new();
    let mut rebuilt_temp_handle = rebuilt_temp_file.reopen().map_err(|e| e.to_string())?;
    rebuilt_temp_handle.read_to_end(&mut rebuilt_content).map_err(|e| e.to_string())?;
    reconstructed_file.write_all(&rebuilt_content).map_err(|e| e.to_string())?;

    println!("Reconstructed file saved as {}", reconstructed_file_path.display());

    // Decrypt the file content from the reconstructed file
    let decrypted_temp_file = decrypt_temp(&rebuilt_temp_file)
        .map_err(|e| format!("Decryption failed: {}", e.to_string()))?;
    println!("Decrypted Content written to temp file");

    // Verify the decrypted content
    let mut decrypted_content = Vec::new();
    let mut decrypted_temp_handle = decrypted_temp_file.reopen().map_err(|e| e.to_string())?;
    decrypted_temp_handle.read_to_end(&mut decrypted_content).map_err(|e| e.to_string())?;
    println!("Decrypted Content matches original: {}", decrypted_content == file_content);

    Ok("File processed, encoded, decoded, encrypted, divided into chunks, uploaded, downloaded, rebuilt, and decrypted successfully".into())
}

fn main() {
    Builder::default()
        .invoke_handler(generate_handler![process_file])
        .run(generate_context!())
        .expect("error while running tauri application");
}
