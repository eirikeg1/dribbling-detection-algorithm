use super::models::VideoData;
use crate::config::Config;
use crate::domain::data::models::Labels;
use rayon::prelude::*; // For parallel processing
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::PathBuf;

// Main Dataset class
#[derive(Clone, Debug)]
pub struct Dataset {
    pub base_dir: PathBuf,
    pub subsets: Vec<String>,
    pub num_cores: usize,
}

impl Dataset {
    // Constructor
    pub fn new(config: Config) -> Self {
        let base_dir = PathBuf::from(&config.data.data_path);
        let subsets = config.data.subsets.clone();
        let num_cores = config.general.num_cores as usize;

        rayon::ThreadPoolBuilder::new()
            .num_threads(num_cores)
            .build_global()
            .unwrap();

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
                    eprintln!(
                        "No labels file found for sequence {:?} in subset {}",
                        seq_dir, subset
                    );
                    return;
                }

                if let Ok(file) = File::open(&labels_file) {
                    let reader = BufReader::new(file);
                    if let Ok(labels) = serde_json::from_reader::<_, Labels>(reader) {
                        let image_id_to_file: HashMap<String, String> = labels
                            .images
                            .into_par_iter()
                            .map(|image| (image.image_id, image.file_name))
                            .collect();

                        labels.annotations.par_iter().for_each(|ann| {
                            let file_name = image_id_to_file
                                .get(&ann.image_id)
                                .cloned()
                                .unwrap_or_else(|| "Unknown".to_string());
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
            Ok(entries) => {
                let vec_entries: Vec<_> =
                    entries
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap_or_else(|err| {
                            eprintln!("Failed to read directory {:?}: {}", subset_dir, err);
                            vec![]
                        });
                vec_entries
            }
            Err(err) => {
                eprintln!("Could not read directory {:?}: {}", subset_dir, err);
                vec![]
            }
        };

        let iter = entries.into_iter().filter_map(move |entry| {
            let seq_dir = entry.path();
            if !seq_dir.is_dir() {
                return None;
            }

            let labels_file = seq_dir.join("Labels-GameState.json");
            if !labels_file.exists() {
                println!("No labels file found for sequence {:?}", seq_dir);
                return None;
            }
            let file = File::open(&labels_file).ok()?;
            let reader = BufReader::new(file);
            let labels: Labels = match serde_json::from_reader(reader) {
                Ok(labels) => labels,
                Err(err) => {
                    eprintln!("Failed to deserialize JSON file {:?}: {}", labels_file, err);
                    return None;
                }
            };

            let image_dir = labels.clone().info.im_dir.unwrap_or("img1".to_string());
            let image_paths: Vec<PathBuf> = labels
                .images
                .iter()
                .map(|image| seq_dir.join(&image_dir).join(&image.file_name))
                .collect();

            Some(Ok(VideoData {
                dir_path: seq_dir,
                image_paths,
                labels: labels,
            }))
        });

        Box::new(iter)
    }
}
