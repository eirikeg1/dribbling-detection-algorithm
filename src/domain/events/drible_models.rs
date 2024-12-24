use std::collections::{HashMap, HashSet};
use std::f64;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub velocity: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct Ball {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_number: u32,
    pub players: Vec<Player>,
    pub ball: Ball,
}

#[derive(Clone, Debug)]
pub struct DribbleEvent {
    pub possession_holder: u32,
    pub start_frame: u32,
    pub end_frame: Option<u32>,
    pub frames: Vec<u32>,
    pub active_defenders: HashSet<u32>,
}

impl DribbleEvent {
    pub fn new(possession_holder: u32, start_frame: u32) -> Self {
        DribbleEvent {
            possession_holder,
            start_frame,
            end_frame: None,
            frames: vec![start_frame],
            active_defenders: HashSet::new(),
        }
    }

    pub fn add_frame(&mut self, frame: u32) {
        self.frames.push(frame);
    }

    pub fn add_defender(&mut self, defender_id: u32) {
        self.active_defenders.insert(defender_id);
    }
}