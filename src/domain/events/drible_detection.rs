use std::collections::HashSet;

use crate::{config::DribblingDetectionConfig, domain::data::models::VideoData};

pub struct DribbleEvent {
    possession_holder: usize,
    start_frame: usize,
    end_frame: Option<usize>,
    defenders: HashSet<usize>,
    frames: Vec<usize>,
}

pub enum DribbleState {
    Search,
    StartTrack,
    TrackClose,
    TrackDuel,
    Detection,
}

pub struct DribbleDetector {
    outer_radius: f64,
    inner_radius: f64,
    min_duration: usize,
}

// impl DribbleDetector {
//     pub fn new(config: &DribblingDetectionConfig) -> Self { ... }

//     pub fn detect_dribbles(&self, video_data: &VideoData) -> Vec<DribbleEvent> {
//         let mut state = DribbleState::Search;
//         let mut dribble_events = Vec::new();

//         for frame in video_data.frames {
//             match state {
//                 DribbleState::Search => { /* Find ball possession */ }
//                 DribbleState::StartTrack => { /* Initialize dribble */ }
//                 DribbleState::TrackClose => { /* Track defenders */ }
//                 DribbleState::TrackDuel => { /* Freeze possession updates */ }
//                 DribbleState::Detection => { /* Detect dribble completion */ }
//             }
//         }

//         dribble_events
//     }
// }
