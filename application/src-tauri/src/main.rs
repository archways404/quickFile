#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{command, Builder, generate_context, generate_handler};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::sync::{Arc, mpsc::{channel, Sender, Receiver}};
use openssl::symm::{Cipher, Crypter, Mode};
use base64::{encode, decode};
use crc32fast::Hasher;
use serde::{Serialize, Deserialize};
use reqwest::Client;
use serde_urlencoded;
use tokio::runtime::Runtime;
use tokio::task;
use futures::future::join_all;
use std::time::Instant;

const CHUNK_SIZE: usize = 1 * 1024 * 1024; // 1MB
const PASSWORD: &str = "your-secure-password";
const SALT: [u8; 16] = [0; 16]; // Replace with a secure random salt

#[derive(Serialize, Deserialize)]
struct PartData {
    lang: String,
    text: String,
    expire: String,
    password: String,
    title: String,
}

fn derive_key(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut key = vec![0u8; 32];
    openssl::pkcs5::pbkdf2_hmac(password.as_bytes(), salt, 100_000, openssl::hash::MessageDigest::sha256(), &mut key).unwrap();
    key
}

fn encrypt(data: &[u8], key: &[u8]) -> String {
    let mut iv = vec![0; 16];
    openssl::rand::rand_bytes(&mut iv).unwrap();
    let mut crypter = Crypter::new(Cipher::aes_256_cbc(), Mode::Encrypt, key, Some(&iv)).unwrap();
    let mut encrypted = vec![0; data.len() + Cipher::aes_256_cbc().block_size()];
    let mut count = crypter.update(data, &mut encrypted).unwrap();
    count += crypter.finalize(&mut encrypted[count..]).unwrap();
    encrypted.truncate(count);
    let mut result = iv.to_vec();
    result.extend_from_slice(&encrypted);
    encode(&result)
}

fn decrypt(encrypted_data: &str, key: &[u8]) -> Vec<u8> {
    let data = decode(encrypted_data).unwrap();
    let (iv, encrypted_text) = data.split_at(16);
    let mut crypter = Crypter::new(Cipher::aes_256_cbc(), Mode::Decrypt, key, Some(iv)).unwrap();
    let mut decrypted = vec![0; encrypted_text.len() + Cipher::aes_256_cbc().block_size()];
    let mut count = crypter.update(encrypted_text, &mut decrypted).unwrap();
    count += crypter.finalize(&mut decrypted[count..]).unwrap();
    decrypted.truncate(count);
    decrypted
}

fn calculate_crc32(data: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(data);
    format!("{:08x}", hasher.finalize())
}

fn split_into_temp_files(data: &[u8], chunk_size: usize) -> Result<Vec<NamedTempFile>, String> {
    let mut temp_files = Vec::new();
    for chunk in data.chunks(chunk_size) {
        let mut temp_file = NamedTempFile::new().map_err(|e| e.to_string())?;
        temp_file.write_all(chunk).map_err(|e| e.to_string())?;
        temp_files.push(temp_file);
    }
    Ok(temp_files)
}

async fn upload_part(client: Arc<Client>, part_path: String, tx: Sender<(usize, String)>, index: usize) {
    let part_content = fs::read_to_string(&part_path).expect("Failed to read part file");
    let part_crc32 = calculate_crc32(part_content.as_bytes());
    let data = PartData {
        lang: "text".to_string(),
        text: part_content.clone(),
        expire: "10m".to_string(),
        password: "".to_string(),
        title: "".to_string(),
    };
    let res = client.post("https://pst.innomi.net/paste/new")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(&data).unwrap())
        .send()
        .await;

    match res {
        Ok(response) => {
            println!("Request to https://pst.innomi.net/paste/new with part content of length {}", data.text.len());
            println!("Part CRC32 before upload: {}", part_crc32);
            println!("Response Status Code: {}", response.status());
            if response.status().is_success() {
                if let Some(title) = response.text().await.ok()
                    .and_then(|body| {
                        let title = body.split("<title>").nth(1)
                            .and_then(|body| body.split("</title>").next())
                            .map(|title| title.split(" - ").next().unwrap_or("").to_string());
                        title
                    }) {
                        tx.send((index, title)).expect("Failed to send link");
                }
            } else {
                println!("Failed to upload part: {}", response.status());
            }
        }
        Err(e) => {
            println!("Error uploading part: {}", e);
        }
    }

    // Delete the part file after upload
    fs::remove_file(part_path).expect("Failed to delete part file");
}

async fn download_part(client: Arc<Client>, url: String, index: usize, tx: Sender<(usize, String)>) {
    if let Ok(response) = client.get(&url).send().await {
        if let Ok(body) = response.text().await {
            if let Some(code_div_content) = body.split(r#"<div class="code" id="code">"#).nth(1)
                .and_then(|body| body.split("</div>").next()) {
                    tx.send((index, code_div_content.to_string())).expect("Failed to send downloaded part");
                    println!("Downloaded part from link: {}", url); // Added logging
            } else {
                println!("Failed to parse part content from link: {}", url); // Added logging
            }
        } else {
            println!("Failed to get response text from link: {}", url); // Added logging
        }
    } else {
        println!("Failed to fetch link: {}", url); // Added logging
    }
}

async fn process_single_file(file_path: &str) -> Result<String, String> {
    let start = Instant::now();
    let file_name = PathBuf::from(file_path).file_name().unwrap().to_str().unwrap().to_string();
    
    // Read the file content
    let file_content = fs::read(file_path).map_err(|e| format!("Failed to read file: {}", e.to_string()))?;

    // Print the file name and size
    println!("Processing file: {}", file_name);
    println!("File size: {} bytes", file_content.len());

    // Base64 encode the file content
    let base64_encoded_data = encode(&file_content);
    println!("Base64 Encoded Content length: {}", base64_encoded_data.len());

    // Base64 decode the content
    let base64_decoded_data = decode(&base64_encoded_data).map_err(|e| format!("Base64 decoding failed: {}", e.to_string()))?;
    println!("Base64 Decoded Content length: {}", base64_decoded_data.len());

    // Encrypt the file content
    let key = derive_key(PASSWORD, &SALT);
    let encrypted_data = encrypt(&file_content, &key);
    println!("Encrypted Content length: {}", encrypted_data.len());

    // Split the encrypted content into 1MB chunks and write to temporary files
    let encrypted_bytes = encrypted_data.as_bytes();
    let temp_files = split_into_temp_files(&encrypted_bytes, CHUNK_SIZE)
        .map_err(|e| format!("Splitting into temp files failed: {}", e.to_string()))?;
    println!("Encrypted Content divided into {} chunks", temp_files.len());

    // Save each chunk to the local file system (optional)
    for (i, temp_file) in temp_files.iter().enumerate() {
        let chunk_path = PathBuf::from(format!("./chunk_{}.txt", i));
        let mut chunk_file = File::create(&chunk_path).map_err(|e| e.to_string())?;
        
        let mut temp_file_content = Vec::new();
        let mut temp_file_handle = temp_file.reopen().map_err(|e| e.to_string())?;
        temp_file_handle.read_to_end(&mut temp_file_content).map_err(|e| e.to_string())?;
        
        chunk_file.write_all(&temp_file_content).map_err(|e| e.to_string())?;
        println!("Saved chunk {} successfully", i);
    }

    // Upload the chunks
    let client = Arc::new(Client::new());
    let (tx, rx): (Sender<(usize, String)>, Receiver<(usize, String)>) = channel();
    let mut handles = vec![];

    for (index, temp_file) in temp_files.iter().enumerate() {
        let client = Arc::clone(&client);
        let tx = tx.clone();
        let part_path = temp_file.path().to_string_lossy().to_string();
        let handle = tokio::spawn(async move {
            upload_part(client, part_path, tx, index).await;
        });
        handles.push(handle);
    }

    join_all(handles).await;

    let mut links: Vec<(usize, String)> = vec![];
    for _ in 0..temp_files.len() {
        if let Ok(link) = rx.recv() {
            links.push(link);
        }
    }
    links.sort_by_key(|k| k.0);
    let formatted_links: Vec<_> = links.into_iter().map(|(index, link)| {
        let part_name = format!("part-{}", index + 1);
        serde_json::json!({ part_name: link })
    }).collect();

    fs::write("response_text.json", serde_json::to_string_pretty(&formatted_links).unwrap()).expect("Failed to save links to file");
    println!("All links have been saved to: response_text.json");

    // Download the chunks
    let (tx, rx): (Sender<(usize, String)>, Receiver<(usize, String)>) = channel();
    let mut handles = vec![];

    for formatted_link in formatted_links.clone() {
        if let Some((part_name, link)) = formatted_link.as_object().unwrap().iter().next() {
            let url = format!("https://pst.innomi.net/paste/{}", link.as_str().unwrap());
            let client = Arc::clone(&client);
            let tx = tx.clone();
            let index = part_name.split('-').nth(1).unwrap().parse::<usize>().unwrap() - 1;
            let handle = tokio::spawn(async move {
                download_part(client, url, index, tx).await;
            });
            handles.push(handle);
        }
    }

    join_all(handles).await;

    let mut downloaded_parts: Vec<(usize, String)> = vec![];
    for _ in 0..formatted_links.len() {
        if let Ok(downloaded_part) = rx.recv() {
            downloaded_parts.push(downloaded_part);
        }
    }
    downloaded_parts.sort_by_key(|k| k.0);

    let combined_base64_content: String = downloaded_parts.into_iter().map(|(_, content)| content).collect();
    println!("Total combined content length: {}", combined_base64_content.len());

    let decrypted_data = decrypt(&combined_base64_content, &key);
    println!("Decrypted data length: {}", decrypted_data.len());

    let combined_crc32 = calculate_crc32(&decrypted_data);
    println!("Combined CRC32: {}", combined_crc32);

    if combined_crc32 != calculate_crc32(&file_content) {
        println!("Error: CRC32 mismatch! Original: {}, Combined: {}", calculate_crc32(&file_content), combined_crc32);
        return Err("CRC32 mismatch".into());
    }

    let reconstructed_file_path = PathBuf::from(format!("reconstructed_file.{}", PathBuf::from(&file_name).extension().and_then(|ext| ext.to_str()).unwrap_or("tmp")));
    let mut reconstructed_file = File::create(&reconstructed_file_path).map_err(|e| e.to_string())?;
    reconstructed_file.write_all(&decrypted_data).map_err(|e| e.to_string())?;
    println!("Reconstructed file saved as {}", reconstructed_file_path.display());

    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);

    Ok("File processed, encoded, decoded, encrypted, divided into chunks, uploaded, downloaded, rebuilt, and decrypted successfully".into())
}

#[command]
async fn process_files(file_paths: Vec<String>) -> Result<String, String> {
    for file_path in file_paths {
        process_single_file(&file_path).await?;
    }
    Ok("All files processed successfully".into())
}

fn main() {
    let rt = Runtime::new().unwrap();
    Builder::default()
        .invoke_handler(generate_handler![process_files])
        .run(generate_context!())
        .expect("error while running tauri application");
}
