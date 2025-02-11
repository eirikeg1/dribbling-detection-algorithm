use std::f64;

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
pub struct DribleFrame {
    pub frame_number: u32,
    pub players: Vec<Player>,
    pub ball: Ball,
}

#[derive(Clone, Debug)]
pub struct DribbleEvent {
    pub finished: bool,
    pub detected_dribble: bool,
    pub detected_tackle: bool, // <-- New field for tackle classification.
    pub ever_contested: bool,
    pub possession_holder: u32,
    pub start_frame: u32,
    pub end_frame: Option<u32>,
    pub frames: Vec<u32>,
    pub active_defenders: Vec<u32>,
    pub inner_defenders: Vec<u32>,
}

impl DribbleEvent {
    pub fn new(possession_holder: u32, start_frame: u32) -> Self {
        DribbleEvent {
            finished: false,
            detected_dribble: false,
            detected_tackle: false, // <-- Initialize new field.
            ever_contested: false,
            possession_holder,
            start_frame,
            end_frame: None,
            frames: vec![start_frame],
            active_defenders: Vec::new(),
            inner_defenders: Vec::new(),
        }
    }

    pub fn add_frame(&mut self, frame: u32) {
        self.frames.push(frame);
    }
}
