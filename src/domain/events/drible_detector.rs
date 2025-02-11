use super::drible_models::{DribbleEvent, DribleFrame, Player};
use std::collections::HashSet;

pub struct DribbleDetector {
    pub outer_rad: f64,
    pub inner_rad: f64,
    pub active_event: Option<DribbleEvent>,
}

impl DribbleDetector {
    pub fn new(inner_rad: f64, outer_rad: f64) -> Self {
        Self {
            outer_rad,
            inner_rad,
            active_event: None,
        }
    }

    /// Returns the Euclidean distance between two points.
    pub fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt()
    }

    /// A static helper that calculates:
    ///   - All defenders (player IDs) within `outer_rad`
    ///   - The subset of those defenders who are also within `inner_rad`
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

    /// Process a frame by either updating an active event or starting a new one.
    pub fn process_frame(&mut self, frame: DribleFrame) -> Option<DribbleEvent> {
        if self.active_event.is_some() {
            self.update_active_event(&frame)
        } else {
            self.try_start_event(&frame)
        }
    }

    /// Try to start a new dribble event.
    /// We start if a player is in possession (ball is within inner_rad) and there is at least one opponent nearby.
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
                event.inner_defenders = inner_defenders;
                self.active_event = Some(event.clone());
                return Some(event);
            }
        }
        None
    }

    /// Update the currently active event based on the new frame.
    fn update_active_event(&mut self, frame: &DribleFrame) -> Option<DribbleEvent> {
        if let Some(ref mut event) = self.active_event {
            // Locate the possession holder.
            if let Some(holder) = frame
                .players
                .iter()
                .find(|p| p.id == event.possession_holder)
            {
                let ball_dist = Self::distance((holder.x, holder.y), (frame.ball.x, frame.ball.y));
                // End event if the ball leaves the inner radius.
                if ball_dist > self.inner_rad {
                    event.end_frame = Some(frame.frame_number);
                    event.finished = true;
                    let finished_event = event.clone();
                    self.active_event = None;
                    return Some(finished_event);
                }

                event.add_frame(frame.frame_number);

                // Recalculate defenders and inner defenders.
                let (defenders, new_inner_defenders) =
                    Self::calc_defenders(&frame.players, holder, self.outer_rad, self.inner_rad);

                // Check if any defender that was previously inside the inner rad is no longer there.
                let previous_inner: HashSet<u32> = event.inner_defenders.iter().cloned().collect();
                let current_inner: HashSet<u32> = new_inner_defenders.iter().cloned().collect();
                if previous_inner.difference(&current_inner).next().is_some() {
                    // A defender left the inner radius: finish event as a successful dribble.
                    event.end_frame = Some(frame.frame_number);
                    event.detected_dribble = true;
                    event.finished = true;
                    let finished_event = event.clone();
                    self.active_event = None;
                    return Some(finished_event);
                }

                // Update the event with the new lists.
                event.inner_defenders = new_inner_defenders;
                event.active_defenders = defenders;
                if !event.inner_defenders.is_empty() {
                    event.ever_contested = true;
                }

                // End event if no defenders remain within the outer radius.
                if event.active_defenders.is_empty() {
                    event.end_frame = Some(frame.frame_number);
                    event.detected_dribble = event.ever_contested;
                    event.finished = true;
                    let finished_event = event.clone();
                    self.active_event = None;
                    return Some(finished_event);
                }

                return Some(event.clone());
            } else {
                // If the possession holder isn’t in the frame, end the event.
                event.end_frame = Some(frame.frame_number);
                event.finished = false;
                let finished_event = event.clone();
                self.active_event = None;
                return Some(finished_event);
            }
        }
        None
    }
}
