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
    pub threshold: f64,
    pub frame_skip: u32,
    pub min_dribble_duration: f64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub data: DataConfig,
    pub dribbling_detection: DribblingDetectionConfig,
}
