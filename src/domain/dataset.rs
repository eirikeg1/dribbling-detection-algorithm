use rayon::prelude::*; // For parallel processing
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
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
pub struct Annotation {
    image_id: String,
    category_id: u32,
    track_id: Option<u32>,
    bbox_image: Option<Value>,
    bbox_pitch: Option<Value>,
    attributes: Option<Attribute>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    role: Option<String>,
    jersey: Option<String>,
    team: Option<String>
}

// Structure to represent the Labels JSON file
#[derive(Debug, Deserialize)]
pub struct Labels {
    images: Vec<Image>,
    annotations: Vec<Annotation>,
}

// Main Dataset class
pub struct Dataset {
    base_dir: PathBuf,
    subsets: Vec<String>,
    num_cores: usize,
}

#[derive(Debug)]
pub struct VideoData {
    pub dir_path: PathBuf,
    pub image_paths: Vec<PathBuf>,
    pub annotations: Vec<Annotation>,
}

impl Dataset {
    // Constructor
    pub fn new(config: Config) -> Self {
        let base_dir = PathBuf::from(&config.data.data_path);
        let subsets = config.data.subsets.clone();
        let num_cores = config.general.num_cores as usize;

        rayon::ThreadPoolBuilder::new().num_threads(num_cores).build_global().unwrap();
        
        Self {
            base_dir,
            subsets,
            num_cores,
        }
    }

    // Method to load data for a given subset
    pub fn load_subset(&self, subset: &str) -> io::Result<()> {
        let subset_dir = self.base_dir.join(subset);
        if !subset_dir.exists() {
            eprintln!("Directory {:?} does not exist.", subset_dir);
            return Ok(());
        }
        
        println!("Loading subset: '{}'", subset);

        fs::read_dir(&subset_dir)?.par_bridge().for_each(|entry| {
            if let Ok(entry) = entry {
                let seq_dir = entry.path();
                if !seq_dir.is_dir() {
                    return;
                }

                let labels_file = seq_dir.join("Labels-GameState.json");
                if !labels_file.exists() {
                    eprintln!("No labels file found for sequence {:?} in subset {}", seq_dir, subset);
                    return;
                }

                if let Ok(file) = File::open(&labels_file) {
                    let reader = BufReader::new(file);
                    if let Ok(labels) = serde_json::from_reader::<_, Labels>(reader) {
                        let image_id_to_file: HashMap<String, String> = labels.images
                            .into_par_iter()
                            .map(|image| (image.image_id, image.file_name))
                            .collect();

                        labels.annotations.par_iter().for_each(|ann| {
                            let file_name = image_id_to_file.get(&ann.image_id).cloned().unwrap_or_else(|| "Unknown".to_string());
                            println!(
                                "Image ID: {}, File Name: {}, Category ID: {}",
                                ann.image_id, file_name, ann.category_id
                            );
                            println!(" * attr: {:?}\n", ann.attributes);
                        });
                    }
                }
            }
        });

        Ok(())
    }

    // Method to load all subsets
    pub fn load_all(&self) -> io::Result<()> {
        println!("Loading all subsets");

        self.subsets.par_iter().for_each(|subset| {
            if let Err(err) = self.load_subset(subset) {
                eprintln!("Error loading subset {}: {}", subset, err);
            }
        });

        println!("Successfully loaded all subsets");
        Ok(())
    }

    // Method to create an iterator for a given subset
    pub fn iter_subset(&self, subset: &str) -> impl Iterator<Item = io::Result<VideoData>> {
        let subset_dir = self.base_dir.join(subset);
        if !subset_dir.exists() {
            return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = io::Result<VideoData>>>;
        }

        let entries = match fs::read_dir(&subset_dir) {
            Ok(entries) => entries.collect::<Result<Vec<_>, _>>().unwrap_or_default(),
            Err(_) => vec![],
        };

        let iter = entries.into_iter().filter_map(move |entry| {
            let seq_dir = entry.path();
            if !seq_dir.is_dir() {
                return None;
            }

            let labels_file = seq_dir.join("Labels-GameState.json");
            if !labels_file.exists() {
                return None;
            }

            let file = File::open(&labels_file).ok()?;
            let reader = BufReader::new(file);
            let labels: Labels = serde_json::from_reader(reader).ok()?;

            let image_paths: Vec<PathBuf> = labels.images
                .iter()
                .map(|image| seq_dir.join("img1").join(&image.file_name))
                .collect();

            Some(Ok(VideoData {
                dir_path: seq_dir,
                image_paths,
                annotations: labels.annotations,
            }))
        });

        Box::new(iter)
    }
}
