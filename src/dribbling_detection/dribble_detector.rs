use std::collections::HashSet;

use super::dribble_models::{DribbleEvent, DribbleFrame, Player};

/// Detects dribble events. An event is started when a defender enters the outer radius,
/// becomes contested if a defender is inside the inner radius for at least `inner_threshold` frames,
/// and is only considered valid if it lasts at least `outer_threshold` frames.
pub struct DribbleDetector {
    pub outer_rad: f64,
    pub inner_rad: f64,
    pub inner_threshold: u32,
    pub outer_threshold: u32,
    
    /// Minimum consecutive frames with at least 1 defender in the outer zone
    pub outer_in_threshold: u32,
    /// Minimum consecutive frames with 0 defenders in the outer zone
    /// before we treat the outer zone as truly "inactive."
    pub outer_out_threshold: u32,

    /// Tracks whether the outer zone is currently considered active 
    outer_zone_active: bool,
    /// Consecutive frames counters for outer zone hysteresis logic.
    consecutive_outer_in: u32,
    consecutive_outer_out: u32,

    // The currently active dribble event (if any).
    pub active_event: Option<DribbleEvent>,
    // Counters for the number of frames defenders have been in the respective zones (once active).
    active_outer_frames: u32,
    active_inner_frames: u32,
}

impl DribbleDetector {
    /// Create a new detector.
    ///
    /// - `inner_rad` and `outer_rad` define the zones.
    /// - `inner_threshold` is how many frames a defender must be in the inner zone (consecutively or total)
    ///   to count as contesting the dribble.
    /// - `outer_threshold` is how many total frames (once outer zone is "active") for an event to be valid.
    /// - `outer_in_threshold` is the consecutive frame count needed to set the outer zone active.
    /// - `outer_out_threshold` is the consecutive frame count needed to set the outer zone inactive.
    pub fn new(
        inner_rad: f64,
        outer_rad: f64,
        inner_threshold: u32,
        outer_threshold: u32,
        outer_in_threshold: u32,
        outer_out_threshold: u32,
    ) -> Self {
        Self {
            inner_rad,
            outer_rad,
            inner_threshold,
            outer_threshold,
            outer_in_threshold,
            outer_out_threshold,
            outer_zone_active: false,
            consecutive_outer_in: 0,
            consecutive_outer_out: 0,
            active_event: None,
            active_outer_frames: 0,
            active_inner_frames: 0,
        }
    }

    pub fn current_active_event(&self) -> Option<&DribbleEvent> {
        self.active_event.as_ref()
    }

    /// Returns the Euclidean distance between two points.
    pub fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt()
    }

    /// Calculates defenders relative to a given possession holder:
    ///   - All defenders (player IDs) within `outer_rad`
    ///   - The subset of those who are also within `inner_rad`
    pub fn calc_defenders(
        players: &[Player],
        holder: &Player,
        outer_rad: f64,
        inner_rad: f64,
    ) -> (Vec<u32>, Vec<u32>) {
        let mut defenders = Vec::new();
        let mut inner_defenders = Vec::new();
        for player in players {
            if player.id != holder.id {
                let d = Self::distance((player.x, player.y), (holder.x, holder.y));
                if d < outer_rad {
                    defenders.push(player.id);
                    if d < inner_rad {
                        inner_defenders.push(player.id);
                    }
                }
            }
        }
        (defenders, inner_defenders)
    }

    /// Top-level entry point: process a frame by either starting a new event
    /// or updating an ongoing event. Returns a completed DribbleEvent, if any finished here.
    pub fn process_frame(&mut self, frame: DribbleFrame) -> Option<DribbleEvent> {
        // 1) Update outer zone state via consecutive-frame hysteresis.
        let defenders_present = self.defenders_in_outer_zone(&frame);
        self.update_outer_zone_state(defenders_present);

        // 2) If the outer zone is "active," proceed with normal logic.
        if self.outer_zone_active {
            if self.active_event.is_some() {
                self.update_active_event(&frame)
            } else {
                self.try_start_event(&frame)
            }
        } else {
            // If the outer zone just turned inactive, any ongoing event ends immediately.
            if let Some(ref mut event) = self.active_event {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }
            None
        }
    }

    /// Check whether we have defenders inside the outer zone for the current frame.
    fn defenders_in_outer_zone(&self, frame: &DribbleFrame) -> bool {
        
        if let Some(holder) = frame
            .players
            .iter()
            .find(|p| Self::distance((p.x, p.y), (frame.ball.x, frame.ball.y)) < self.inner_rad)
        {
            let (defenders, _inner_defenders) =
                Self::calc_defenders(&frame.players, holder, self.outer_rad, self.inner_rad);
            return !defenders.is_empty();
        }
        false
    }

    /// Update the "outer zone active" state using consecutive-frame counters.
    fn update_outer_zone_state(&mut self, defenders_present: bool) {
        if defenders_present {
            self.consecutive_outer_in += 1;
            self.consecutive_outer_out = 0;
        } else {
            self.consecutive_outer_out += 1;
            self.consecutive_outer_in = 0;
        }

        // If not active, check if we have enough consecutive "in" frames to activate it.
        if !self.outer_zone_active && self.consecutive_outer_in >= self.outer_in_threshold {
            self.outer_zone_active = true;
            // Once we become active, reset the counters or let them continue
            // depending on your preference:
            // self.consecutive_outer_in = 0;
            // self.consecutive_outer_out = 0;
        }

        // If active, check if we have enough consecutive "out" frames to deactivate it.
        if self.outer_zone_active && self.consecutive_outer_out >= self.outer_out_threshold {
            self.outer_zone_active = false;
        }
    }

    /// Attempt to start a new dribble event if:
    /// - There's a ball holder (distance to ball < inner_rad),
    /// - The outer zone is active (which we handle via hysteresis above).
    fn try_start_event(&mut self, frame: &DribbleFrame) -> Option<DribbleEvent> {
        
        if let Some(holder) = frame
            .players
            .iter()
            .find(|p| Self::distance((p.x, p.y), (frame.ball.x, frame.ball.y)) < self.inner_rad)
        {
            let (defenders, inner_defenders) =
                Self::calc_defenders(&frame.players, holder, self.outer_rad, self.inner_rad);
            
            if !defenders.is_empty() {
                let mut event = DribbleEvent::new(holder.id, frame.frame_number);
                event.active_defenders = defenders;
                event.inner_defenders = inner_defenders.clone();
                self.active_event = Some(event.clone());

                // Initialize counters.
                self.active_outer_frames = 1;
                self.active_inner_frames = if !inner_defenders.is_empty() { 1 } else { 0 };

                return Some(event);
            }
        }
        None
    }

    /// Update the currently active event using the new frame.
    fn update_active_event(&mut self, frame: &DribbleFrame) -> Option<DribbleEvent> {
        if let Some(ref mut event) = self.active_event {
            // Retrieve the current possession holder.
            let old_holder = match frame
                .players
                .iter()
                .find(|p| p.id == event.possession_holder)
            {
                Some(holder) => holder,
                None => {
                    // Possession holder not in frame: end event.
                    event.end_frame = Some(frame.frame_number);
                    event.finished = false;
                    return self.finalize_event(frame.frame_number);
                }
            };

            let old_holder_ball_dist =
                Self::distance((old_holder.x, old_holder.y), (frame.ball.x, frame.ball.y));

            // Recalculate defenders for counters.
            let (defenders, new_inner_defenders) =
                Self::calc_defenders(&frame.players, old_holder, self.outer_rad, self.inner_rad);

            // Increment counters if defenders are present.
            if !defenders.is_empty() {
                self.active_outer_frames += 1;
            }
            if !new_inner_defenders.is_empty() {
                self.active_inner_frames += 1;
            }

            // Check if another candidate (a defender) has the ball inside the inner zone.
            if let Some(_candidate) = frame.players.iter().find(|p| {
                p.id != event.possession_holder
                    && Self::distance((p.x, p.y), (frame.ball.x, frame.ball.y)) < self.inner_rad
            }) {
                if old_holder_ball_dist > self.inner_rad {
                    // Possession change.
                    event.end_frame = Some(frame.frame_number);
                    event.finished = true;
                    // Only count the event as contested if the defender was in the inner zone long enough.
                    if self.active_inner_frames >= self.inner_threshold {
                        event.detected_tackle = true;
                    } else {
                        event.detected_dribble = true;
                    }
                    return self.finalize_event(frame.frame_number);
                }
            }

            // If the original holder loses the ball (outside the inner zone), end event.
            if old_holder_ball_dist > self.inner_rad {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }

            // Continue the event.
            event.add_frame(frame.frame_number);

            // If any defender previously in inner zone is now gone, finish the event.
            let previous_inner: HashSet<u32> = event.inner_defenders.iter().cloned().collect();
            let current_inner: HashSet<u32> = new_inner_defenders.iter().cloned().collect();
            if previous_inner.difference(&current_inner).next().is_some() {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                // Only count the event as contested if the defender was in the inner zone long enough.
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }

            event.inner_defenders = new_inner_defenders;
            event.active_defenders = defenders;

            // If defenders have been in the inner zone enough frames, mark it contested.
            if !event.inner_defenders.is_empty() && self.active_inner_frames >= self.inner_threshold {
                event.ever_contested = true;
            }

            // End the event if no outer defenders remain (or if `outer_zone_active` turned false).
            if event.active_defenders.is_empty() {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }

            Some(event.clone())
        } else {
            None
        }
    }

    /// Finalize the active event. The event is accepted only if the total frames with defenders
    /// in the outer zone (active_outer_frames) meets `outer_threshold`. Then it resets state.
    fn finalize_event(&mut self, frame_number: u32) -> Option<DribbleEvent> {
        if let Some(ref mut event) = self.active_event {
            event.end_frame = Some(frame_number);
            // Check that the event lasted at least as long as required.
            if self.active_outer_frames < self.outer_threshold {
                self.reset_active_event();
                return None;
            }
            let finished_event = event.clone();
            self.reset_active_event();
            return Some(finished_event);
        }
        None
    }

    fn reset_active_event(&mut self) {
        self.active_event = None;
        self.active_outer_frames = 0;
        self.active_inner_frames = 0;
    }
}
