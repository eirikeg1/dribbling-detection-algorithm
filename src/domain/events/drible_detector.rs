use std::collections::HashSet;

use super::drible_models::{DribbleEvent, DribleFrame, Player};

/// Detects dribble events. An event is started when a defender enters the outer radius,
/// becomes contested if a defender is inside the inner radius for at least `inner_threshold` frames,
/// and is only considered valid if it lasts at least `outer_threshold` frames.
pub struct DribbleDetector {
    pub outer_rad: f64,
    pub inner_rad: f64,
    pub inner_threshold: u32,
    pub outer_threshold: u32,
    pub active_event: Option<DribbleEvent>,
    // Counters for the number of frames defenders have been in the respective zones.
    active_outer_frames: u32,
    active_inner_frames: u32,
}

impl DribbleDetector {
    /// Create a new detector.
    ///
    /// - `inner_rad` and `outer_rad` are the radii defining the zones.
    /// - `inner_threshold` is the minimum number of frames that a defender must be in the inner zone
    ///    to count as contesting the dribble.
    /// - `outer_threshold` is the minimum total number of frames (starting when a defender enters the outer zone)
    ///    for an event to be valid.
    pub fn new(inner_rad: f64, outer_rad: f64, inner_threshold: u32, outer_threshold: u32) -> Self {
        Self {
            inner_rad,
            outer_rad,
            inner_threshold,
            outer_threshold,
            active_event: None,
            active_outer_frames: 0,
            active_inner_frames: 0,
        }
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

    /// Process a frame by either starting a new event or updating an ongoing event.
    pub fn process_frame(&mut self, frame: DribleFrame) -> Option<DribbleEvent> {
        if self.active_event.is_some() {
            self.update_active_event(&frame)
        } else {
            self.try_start_event(&frame)
        }
    }

    /// Try to start a new dribble event.
    ///
    /// We start if a player is in possession (ball is within the inner radius) and at least one opponent
    /// is in the outer zone.
    fn try_start_event(&mut self, frame: &DribleFrame) -> Option<DribbleEvent> {
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
                // Initialize the counters.
                self.active_outer_frames = 1;
                self.active_inner_frames = if !inner_defenders.is_empty() { 1 } else { 0 };
                return Some(event);
            }
        }
        None
    }

    /// Update the currently active event using the new frame.
    fn update_active_event(&mut self, frame: &DribleFrame) -> Option<DribbleEvent> {
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

            // Recalculate defenders.
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
                    // Only count the event as contested (tackle) if the defender was in the inner zone long enough.
                    if self.active_inner_frames >= self.inner_threshold {
                        event.detected_tackle = true;
                    } else {
                        event.detected_dribble = true;
                    }
                    return self.finalize_event(frame.frame_number);
                }
            }

            // If the original holder loses the ball (i.e. is outside the inner zone), finish the event.
            if old_holder_ball_dist > self.inner_rad {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }

            // Continue the event.
            event.add_frame(frame.frame_number);

            // If any defender previously in the inner zone is now gone, finish the event.
            let previous_inner: HashSet<u32> = event.inner_defenders.iter().cloned().collect();
            let current_inner: HashSet<u32> = new_inner_defenders.iter().cloned().collect();
            if previous_inner.difference(&current_inner).next().is_some() {
                event.end_frame = Some(frame.frame_number);
                event.finished = true;
                // Even if a defender dipped into the inner zone, if they weren’t there long enough
                // we treat the event as a dribble.
                event.detected_dribble = true;
                return self.finalize_event(frame.frame_number);
            }

            // Update the lists of defenders.
            event.inner_defenders = new_inner_defenders;
            event.active_defenders = defenders;
            if !event.inner_defenders.is_empty() && self.active_inner_frames >= self.inner_threshold
            {
                event.ever_contested = true;
            }

            // End the event if no outer defenders remain.
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

    /// Finalize the active event.
    ///
    /// The event is accepted only if the total frames with defenders in the outer zone
    /// (active_outer_frames) meets or exceeds `outer_threshold`. In all cases the event’s
    /// `end_frame` is set and the detector state is reset.
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

    /// Reset the active event and frame counters.
    fn reset_active_event(&mut self) {
        self.active_event = None;
        self.active_outer_frames = 0;
        self.active_inner_frames = 0;
    }
}
