use super::models::{Annotation, DribbleEventsExport, DribbleLabel, VideoData};
use crate::config::Config;
use crate::data::models::Labels;
use opencv::core::Tuple;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Dataset {
    pub base_dir: PathBuf,
    pub subsets: Vec<String>,
    pub num_cores: usize,
    pub config: Config,
}

fn parse_frame_number(file_name: &str) -> Option<u32> {
    // "000123.jpg" => "000123" => parse u32 => 123
    let trimmed = file_name.trim_end_matches(".jpg");
    trimmed.parse::<u32>().ok()
}

/// Helper to load dribble-events map if in review mode.
/// Returns a HashMap<video_id, HashSet<frame_nums_in_events>> or None if not in review mode.
pub fn load_dribble_events_map(config: &Config) -> Option<HashMap<String, Vec<(u32,u32)>>> {
    let review_mode = config.general.review_mode.unwrap_or(false);
    if !review_mode {
        return None;
    }

    // Attempt to read dribble_events.json
    let dribble_events_path = &config.data.dribble_events_path;
    let dribble_json = match fs::read_to_string(dribble_events_path) {
        Ok(txt) => txt,
        Err(e) => {
            eprintln!("Could not read dribble events file: {e}");
            return None;
        }
    };

    // Attempt to parse JSON
    let dribble_events: DribbleEventsExport = match serde_json::from_str(&dribble_json) {
        Ok(obj) => obj,
        Err(e) => {
            eprintln!("Could not parse dribble_events.json: {e}");
            return None;
        }
    };

    // Build our video->frames map
    let mut video_to_valid_frames = HashMap::new();
    for video_entry in &dribble_events.videos {
        // If no dribble events, the resulting set is empty

        // Insert all event frames
        for event in &video_entry.dribble_events {
            if !video_to_valid_frames.contains_key(&video_entry.video_id) {
                video_to_valid_frames.insert(video_entry.video_id.clone(), vec![]);
            }
            let start = event.start_frame;
            let end = event.end_frame.unwrap_or(start);
            video_to_valid_frames.get_mut(&video_entry.video_id).unwrap().push((start, end));
        }
    }

    Some(video_to_valid_frames)
}

impl Dataset {
    pub fn new(config: Config) -> Self {
        let base_dir = PathBuf::from(&config.data.data_path);
        let subsets = config.data.subsets.clone();
        let num_cores = config.general.num_cores as usize;

        Self {
            base_dir,
            subsets,
            num_cores,
            config,
        }
    }

    // Create an iterator for a specific subset, ordered alphabetically
     /// Create an iterator for a specific subset, ordered alphabetically.
    /// This version filters out videos that have no dribble events
    /// and filters out any frames not in the dribble-event ranges.
    pub fn iter_subset(&self, subset: &str) -> impl Iterator<Item = io::Result<VideoData>> {
        let subset_dir = self.base_dir.join(subset);
        if !subset_dir.exists() {
            return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = io::Result<VideoData>>>;
        }

        // Read and collect entries (directories) under this subset
        let mut entries = match fs::read_dir(&subset_dir) {
            Ok(dir_entries) => dir_entries.filter_map(|e| e.ok()).collect::<Vec<_>>(),
            Err(err) => {
                eprintln!("Could not read directory {:?}: {}", subset_dir, err);
                vec![]
            }
        };

        // Sort entries alphabetically
        entries.sort_by(|a, b| a.path().cmp(&b.path()));

        // Create an iterator producing VideoData
        let iter = entries.into_iter().filter_map(move |entry| {
            let seq_dir = entry.path();
            if !seq_dir.is_dir() {
                return None;
            }

            // The directory name is assumed to be "video_id" (e.g. "video_81")
            let video_id = match seq_dir.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => return None,
            };

            let labels_file = seq_dir.join("Labels-GameState.json");
            if !labels_file.exists() {
                println!("No labels file found for sequence {:?}", seq_dir);
                return None;
            }

            // Parse the Labels JSON
            let file = File::open(&labels_file).ok()?;
            let reader = BufReader::new(file);
            let mut labels: Labels = match serde_json::from_reader(reader) {
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

            // Return our VideoData
            Some(Ok(VideoData {
                dir_path: seq_dir,
                image_paths,
                labels,
            }))
        });

        Box::new(iter)
    }
}