use std::env;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct GeneralConfig {
    pub num_cores: u32,
    pub log_level: String,
    pub video_mode: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DataConfig {
    pub data_path: String,
    pub subsets: Vec<String>,
    pub output_path: String,
    pub huggingface_dataset_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DribblingDetectionConfig {
    pub outer_threshold: u32,
    pub inner_threshold: u32,
    pub frame_skip: u32,
    pub min_duration: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub ignore_person_classes: bool,
    pub ignore_teams: bool,
    pub outer_in_threshold: u32,
    pub outer_out_threshold: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VisualizationConfig {
    pub autoplay: bool,
    pub scale_factor: f64,
    pub minimap_x: i32,
    pub minimap_y: i32,
    pub minimap_width: i32,
    pub minimap_height: i32,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub data: DataConfig,
    pub dribbling_detection: DribblingDetectionConfig,
    pub visualization: VisualizationConfig,
}

impl Config {
    /// Applies overrides from environment variables, if set.
    ///
    /// - `DATA_PATH`: overrides `data.data_path`
    /// - `OUTPUT_PATH`: overrides `data.output_path`
    ///
    pub fn apply_env_overrides(mut self) -> Self {
        println!("applying existing env overrides");
        if let Ok(dp) = env::var("DATA_PATH") {
            println!("Overriding data path: {}", dp);
            self.data.data_path = dp;
        }
        if let Ok(op) = env::var("OUTPUT_PATH") {
            println!("Overriding output path: {}", op);
            self.data.output_path = op;
        }
        self
    }
}
