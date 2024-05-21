#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{command, Builder, generate_context, generate_handler};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[command]
fn process_file(file_name: String, file_content: Vec<u8>) -> Result<String, String> {
    // Print the file name and size
    println!("Received file: {}", file_name);
    println!("File size: {} bytes", file_content.len());

    let path = PathBuf::from(format!("./{}", file_name));
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&file_content).map_err(|e| e.to_string())?;
    println!("File processed successfully");
    Ok("File processed successfully".into())
}

fn main() {
    Builder::default()
        .invoke_handler(generate_handler![process_file])
        .run(generate_context!())
        .expect("error while running tauri application");
}
