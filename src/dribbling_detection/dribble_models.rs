use std::f64;

use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub velocity: (f64, f64),
    pub within_inner_rad: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Ball {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct DribbleFrame {
    pub frame_number: u32,
    pub players: Vec<Player>,
    pub ball: Ball,
}

#[derive(Clone, Debug, Serialize)]
pub struct DribbleEvent {
    pub file_name: String,
    pub finished: bool,
    pub detected_dribble: bool,
    pub detected_tackle: bool,
    pub ever_contested: bool,
    pub possession_holder: u32,
    pub start_frame: u32,
    pub end_frame: Option<u32>,
    pub frames: Vec<u32>,
    pub active_defenders: Vec<u32>,
    pub inner_defenders: Vec<u32>,
    pub ball_between_occurred: bool,
}

impl DribbleEvent {
    pub fn new(possession_holder: u32, start_frame: u32, file_name: String) -> Self {
        DribbleEvent {
            file_name,
            finished: false,
            detected_dribble: false,
            detected_tackle: false,
            ever_contested: false,
            possession_holder,
            start_frame,
            end_frame: None,
            frames: vec![start_frame],
            active_defenders: Vec::new(),
            inner_defenders: Vec::new(),

            // Initialize new field to false
            ball_between_occurred: false,
        }
    }

    pub fn add_frame(&mut self, frame: u32) {
        self.frames.push(frame);
    }

    /// Extends the defenders and frames with the values of another dribble event.
    pub fn extend(&mut self, other: &DribbleEvent) {
        self.frames.extend(&other.frames);
        self.active_defenders.extend(&other.active_defenders);
        self.inner_defenders.extend(&other.inner_defenders);
        // If the other event had the ball_between_occurred flag set, carry it over.
        if other.ball_between_occurred {
            self.ball_between_occurred = true;
        }
    }
}
