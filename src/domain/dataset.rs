use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::Path;
use serde::Deserialize;
use serde_json::Value;

use crate::config::Config;

// Structure to represent an Image
#[derive(Debug, Deserialize)]
struct Image {
    image_id: String,
    file_name: String,
    height: u32,
    width: u32,
}

// Structure to represent an Annotation
#[derive(Debug, Deserialize)]
struct Annotation {
    image_id: String,
    category_id: u32,
    track_id: Option<u32>,
    bbox_image: Option<Value>,
    bbox_pitch: Option<Value>,
}

// Structure to represent the Labels JSON file
#[derive(Debug, Deserialize)]
struct Labels {
    images: Vec<Image>,
    annotations: Vec<Annotation>,
}

// Main Dataset class
pub struct Dataset {
    base_dir: String,
    subsets: Vec<String>,
    config: Config,
}

impl Dataset {
    // Constructor
    pub fn new(config: Config) -> Self {
        let base_dir = &config.data.data_path;
        let subsets = &config.data.subsets;
        Self {
            base_dir: base_dir.to_string(),
            subsets: subsets.into_iter().map(|s| s.to_string()).collect(),
            config: config,
        }
    }

    // Method to load data for a given subset
    pub fn load_subset(&self, subset: &str) -> io::Result<()> {
        let subset_dir = Path::new(&self.base_dir).join(subset);
        if !subset_dir.exists() {
            println!("Directory {:?} does not exist.", subset_dir);
            return Ok(());
        }
        
        println!("Loading subset: '{}'", subset);
        // List all sequences in the subset
        for entry in fs::read_dir(subset_dir)? {
            let seq_dir = entry?.path();
            if !seq_dir.is_dir() {
                continue;
            }

            let images_dir = seq_dir.join("img1");
            let labels_file = seq_dir.join("Labels-GameState.json");

            if labels_file.exists() {
                let file = File::open(&labels_file)?;
                let reader = BufReader::new(file);
                let labels: Labels = serde_json::from_reader(reader)?;

                // Create a mapping from image_id to file_name
                let image_id_to_file: HashMap<String, String> = labels
                    .images
                    .iter()
                    .map(|image| (image.image_id.clone(), image.file_name.clone()))
                    .collect();

                // Process annotations
                for ann in &labels.annotations {
                    let unknown = "Unknown".to_string();
                    let file_name = image_id_to_file
                        .get(&ann.image_id)
                        .unwrap_or(&unknown);
                    // println!(
                    //     "Image ID: {}, File Name: {}, Category ID: {}",
                    //     ann.image_id, file_name, ann.category_id
                    // );
                }
            } else {
                println!("No labels file found for sequence {:?} in subset {}", seq_dir, subset);
            }
        }

        Ok(())
    }

    // Method to load all subsets
    pub fn load_all(&self) -> io::Result<()> {
        println!("Loading all subsets");
        for subset in &self.subsets {
            self.load_subset(subset)?;
        }
        println!("Suscessfully loaded all subsets");
        Ok(())
    }
}
