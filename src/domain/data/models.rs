use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

// Structure to represent an Image
#[derive(Clone, Debug, Deserialize)]
pub struct Image {
    pub image_id: String,
    pub file_name: String,
    pub height: u32,
    pub width: u32,
}

// Represents the image-space bounding box
#[derive(Clone, Debug, Deserialize)]
pub struct BboxImage {
    pub x: f64,
    pub y: f64,
    pub x_center: f64,
    pub y_center: f64,
    pub w: f64,
    pub h: f64,
}

// Represents the pitch-space bounding box
#[derive(Clone, Debug, Deserialize)]
pub struct BboxPitch {
    pub x_bottom_left: f64,
    pub y_bottom_left: f64,
    pub x_bottom_right: f64,
    pub y_bottom_right: f64,
    pub x_bottom_middle: f64,
    pub y_bottom_middle: f64,
}

// Represents the raw pitch-space bounding box (if available)
#[derive(Clone, Debug, Deserialize)]
pub struct BboxPitchRaw {
    pub x_bottom_left: f64,
    pub y_bottom_left: f64,
    pub x_bottom_right: f64,
    pub y_bottom_right: f64,
    pub x_bottom_middle: f64,
    pub y_bottom_middle: f64,
}

// A structure for line points associated with pitch markings
#[derive(Clone, Debug, Deserialize)]
pub struct LinePoint {
    pub x: f64,
    pub y: f64,
}

// Structure to represent an Annotation
#[derive(Clone, Debug, Deserialize)]
pub struct Annotation {
    pub image_id: String,
    pub category_id: u32,
    pub track_id: Option<u32>,
    pub bbox_image: Option<BboxImage>,
    pub bbox_pitch: Option<BboxPitch>,
    pub bbox_pitch_raw: Option<BboxPitchRaw>,
    pub attributes: Option<Attribute>,
    #[serde(default)]
    pub lines: Option<HashMap<String, Vec<LinePoint>>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Attribute {
    pub role: Option<String>,
    pub jersey: Option<String>,
    pub team: Option<String>,
}

// Structure to represent the Labels JSON file
#[derive(Clone, Debug, Deserialize)]
pub struct Labels {
    pub images: Vec<Image>,
    pub annotations: Vec<Annotation>,
}

#[derive(Clone, Debug)]
pub struct VideoData {
    pub dir_path: PathBuf,
    pub image_paths: Vec<PathBuf>,
    pub labels: Labels,
}
