use futures_util::StreamExt;
use reqwest::Client;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use zip::read::ZipArchive;

use crate::config::Config;

///////////////////////////////////////////////////////////
////////////  Notice: This is not working yet  ///////////
/////////////////////////////////////////////////////////

/// Download large file in chunks and save it to disk
async fn download_large_file(url: &str, local_path: &str) {
    // Ensure parent directories exist for the local path
    if let Some(parent) = Path::new(local_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
    }

    let response = Client::new()
        .get(url)
        .send()
        .await
        .expect("Failed to send download request");

    let status = response.status();
    if !status.is_success() {
        let error_message = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to get error message".to_string());
        panic!(
            "Failed to download file. Status: {}. Message: {}",
            status, error_message
        );
    }

    let mut file = TokioFile::create(local_path)
        .await
        .expect("Failed to create local file");
    let mut stream = response.bytes_stream();

    let mut total_downloaded = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("Error while downloading file chunk");
        total_downloaded += chunk.len();
        file.write_all(&chunk)
            .await
            .expect("Failed to write chunk to file");

        if total_downloaded % (1024 * 1024) == 0 {
            println!("Downloaded {} MB...", total_downloaded / (1024 * 1024));
        }
    }

    // Ensure all writes are fully flushed before moving on
    file.flush().await.expect("Failed to flush file");
    println!(
        "Download complete: {} ({} MB)",
        local_path,
        total_downloaded / (1024 * 1024)
    );
}

/// Extracts a ZIP file from disk and saves contents to a directory
fn extract_zip(zip_path: &str, output_dir: &str) -> io::Result<()> {
    let mut file = File::open(zip_path)?;

    // Validate if the file is a proper ZIP archive
    let mut buffer = [0; 2];
    file.read_exact(&mut buffer)?;
    if &buffer != b"PK" {
        panic!(
            "The file is not a valid ZIP archive. First two bytes: {:?}",
            buffer
        );
    }

    let mut archive = ZipArchive::new(File::open(zip_path)?)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = Path::new(output_dir).join(file.name());

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out_path)?;
            io::copy(&mut file, &mut outfile)?;
        }
        println!("Extracted: {}", out_path.display());
    }

    Ok(())
}

/// Downloads and extracts a large dataset using the config information
pub async fn download_and_extract_dataset(config: &Config) {
    let data_path = &config.data.data_path;
    let output_path = &config.data.output_path;

    // 1. Ensure the directory exists
    if !Path::new(data_path).exists() {
        fs::create_dir_all(data_path).expect("Failed to create local directory");
    }

    for subset in &config.data.subsets {
        let zip_file_path = format!("{}/{}_dataset.zip", data_path, subset);
        let dataset_url = format!("{}&split={}", config.data.huggingface_dataset_url, subset);

        // 2. Download large ZIP file (streamed directly to disk)
        println!(
            "Starting download of {} dataset from: {}",
            subset, dataset_url
        );
        download_large_file(&dataset_url, &zip_file_path).await;

        // 3. Validate and Extract the ZIP file to the output directory
        println!("Extracting ZIP file: {}", zip_file_path);
        match extract_zip(&zip_file_path, output_path) {
            Ok(_) => println!("Successfully extracted ZIP file: {}", zip_file_path),
            Err(e) => panic!("Failed to extract ZIP file. Error: {:?}", e),
        }

        // 4. Clean up the ZIP file after extraction
        println!("Cleaning up temporary ZIP file...");
        fs::remove_file(&zip_file_path).expect("Failed to remove ZIP file");
    }

    println!("All datasets downloaded and extracted successfully.");
}
