use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

// Structure to represent an Image
#[derive(Debug, Deserialize)]
pub struct Image {
    pub image_id: String,
    pub file_name: String,
    pub height: u32,
    pub width: u32,
}

// Structure to represent an Annotation
#[derive(Debug, Deserialize)]
pub struct Annotation {
    pub image_id: String,
    pub category_id: u32,
    pub track_id: Option<u32>,
    pub bbox_image: Option<Value>,
    pub bbox_pitch: Option<Value>,
    pub attributes: Option<Attribute>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    pub role: Option<String>,
    pub jersey: Option<String>,
    pub team: Option<String>,
}

// Structure to represent the Labels JSON file
#[derive(Debug, Deserialize)]
pub struct Labels {
    pub images: Vec<Image>,
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct VideoData {
    pub dir_path: PathBuf,
    pub image_paths: Vec<PathBuf>,
    pub annotations: Vec<Annotation>,
}
