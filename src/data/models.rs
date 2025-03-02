use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::dribbling_detection::dribble_models::DribbleEvent;

// This struct holds some metadata similar to your input's "info" field.
#[derive(Serialize)]
pub struct ExportInfo {
    pub version: String,
    pub generated_at: String,
}

#[derive(Serialize)]
pub struct DribbleLabel {
    pub finished: bool,
    pub detected_dribble: bool,
    pub detected_tackle: bool,
    pub ever_contested: bool,
    pub possession_holder: u32,
    pub start_frame: u32,
    pub end_frame: Option<u32>,
}

impl From<&DribbleEvent> for DribbleLabel {
    fn from(event: &DribbleEvent) -> Self {
        DribbleLabel {
            finished: event.finished,
            detected_dribble: event.detected_dribble,
            detected_tackle: event.detected_tackle,
            ever_contested: event.ever_contested,
            possession_holder: event.possession_holder,
            start_frame: event.start_frame,
            end_frame: event.end_frame,
        }
    }
}

// Each video’s dribble events are stored here.
#[derive(Serialize)]
pub struct VideoDribbleEvents {
    pub video_id: String,
    pub file_name: String,
    pub dribble_events: Vec<DribbleLabel>,
}

// This is the top-level export pub.
#[derive(Serialize)]
pub struct DribbleEventsExport {
    pub info: ExportInfo,
    pub videos: Vec<VideoDribbleEvents>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum SpecialHighlight {
    PossesionHolder,
    Defender,
    Ball,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Info {
    pub version: String,
    pub game_id: Option<String>,
    pub num_tracklets: Option<String>,
    pub action_position: Option<String>,
    pub action_class: Option<String>,
    pub visibility: Option<String>,
    pub game_time_start: Option<String>,
    pub game_time_stop: Option<String>,
    pub clip_start: String,
    pub clip_stop: String,
    pub name: String,
    pub im_dir: Option<String>,
    pub frame_rate: f32,
    pub seq_length: u32,
    pub im_ext: String,
}

// Structure to represent an Image
#[derive(Clone, Debug, Deserialize)]
pub struct Image {
    pub is_labeled: bool,
    pub image_id: String,
    pub file_name: String,
    pub height: u32,
    pub width: u32,
    pub has_labeled_person: Option<bool>,
    pub has_labeled_pitch: Option<bool>,
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
#[derive(Clone, Debug, Deserialize, Default)]
pub struct Annotation {
    pub id: String,
    pub image_id: String,
    pub track_id: Option<u32>,
    pub supercategory: String,
    pub category_id: u32,
    pub bbox_image: Option<BboxImage>,
    pub bbox_pitch: Option<BboxPitch>,
    pub bbox_pitch_raw: Option<BboxPitchRaw>,
    pub attributes: Option<Attribute>,
    #[serde(default)]
    pub lines: Option<HashMap<String, Vec<LinePoint>>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Attribute {
    pub role: Option<String>,
    pub jersey: Option<String>,
    pub team: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Category {
    pub supercategory: String,
    pub id: u32,
    pub name: String,
    pub lines: Option<Vec<String>>,
}

// Structure to represent the Labels JSON file
#[derive(Clone, Debug, Deserialize)]
pub struct Labels {
    pub info: Info,
    pub images: Vec<Image>,
    pub annotations: Vec<Annotation>,
    pub categories: Vec<Category>,
}

#[derive(Clone, Debug)]
pub struct VideoData {
    pub dir_path: PathBuf,
    pub image_paths: Vec<PathBuf>,
    pub labels: Labels,
}
