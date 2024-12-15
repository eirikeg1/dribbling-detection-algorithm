use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GeneralConfig {
    pub data_path: String,
    pub num_cores: u32,
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct DribblingDetectionConfig {
    pub threshold: f64,
    pub max_frame_skip: u32,
    pub min_dribble_duration: f64,
}

#[derive(Debug, Deserialize)]
pub struct PathsConfig {
    pub output_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub dribbling_detection: DribblingDetectionConfig,
    pub paths: PathsConfig,
}
