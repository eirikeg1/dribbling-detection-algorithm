use super::drible_models::{Ball, DribbleEvent, DribleFrame, Player};

pub struct DribbleDetector {
    pub outer_rad: f64,
    pub inner_rad: f64,
    pub active_event: Option<DribbleEvent>,
}

impl DribbleDetector {
    pub fn new(inner_rad: f64, outer_rad: f64) -> Self {
        DribbleDetector {
            outer_rad,
            inner_rad,
            active_event: None,
        }
    }

    pub fn distance(p1: (f64, f64), p2: (f64, f64)) -> f64 {
        ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt()
    }

    /// Finds the closest player to the ball, if any.
    /// (Does not check any distance threshold.)
    pub fn closest_player_to_ball<'a>(
        &self,
        players: &'a [Player],
        ball: &Ball,
    ) -> Option<&'a Player> {
        players.iter().min_by(|p1, p2| {
            let d1 = Self::distance((p1.x, p1.y), (ball.x, ball.y));
            let d2 = Self::distance((p2.x, p2.y), (ball.x, ball.y));
            d1.partial_cmp(&d2).unwrap()
        })
    }

    /// Find which players are within outer_rad of the given possession holder
    pub fn detect_defenders<'a>(
        &self,
        players: &'a [Player],
        possession_holder: &Player,
    ) -> Vec<&'a Player> {
        players
            .iter()
            .filter(|player| {
                // skip the holder themselves
                player.id != possession_holder.id
                    && Self::distance(
                        (player.x, player.y),
                        (possession_holder.x, possession_holder.y),
                    ) < self.outer_rad
            })
            .collect()
    }

    /// Called once per frame. 
    /// - If there's an active event, update/end it.
    /// - If an event ends, optionally check if a new one starts.
    /// - If no event is active, see if a new event starts.
    pub fn process_frame(&mut self, frame: DribleFrame) -> Option<DribbleEvent> {

        // println!("Current event state: {:?}", self.active_event);
        let mut event = self.active_event.clone();
        if event.is_some() {
            let mut event = event.take().unwrap();
            // There is a currently active event
            let completed_event = self.handle_active_event(&frame, &mut event).unwrap();
            if completed_event.finished {
                // The old event ended. 
                println!("Ended event: {:?}", completed_event);

                // Optionally check if a new event starts THIS FRAME:
                if let Some(new_event) = self.handle_search_state(&frame) {
                    // Return the new event (or you could return the old + new, depending on your design)
                    return Some(new_event);
                }

                // If no new event started, just return the one that ended
                return Some(completed_event);
            } else {
                println!("Continuing event: {:?}", event);
                // The event is still active
                self.active_event = Some(event);
                return self.active_event.clone();
            }
        } else {
            // No active event -> try to start a new one
            println!("No active event. Searching for new event...");
            self.active_event = None;
            if let Some(new_event) = self.handle_search_state(&frame) {
                return Some(new_event);
            }
            // Otherwise, do nothing
            None
        }
    }

    /// If no event is active, see if a new dribble event can start.
    /// (We only start if at least one player is within inner_rad of the ball.)
    fn handle_search_state(&mut self, frame: &DribleFrame) -> Option<DribbleEvent> {
        // Find who is closest to the ball
        println!("\n\nSearching for new event...");
        if let Some(closest_player) = self.closest_player_to_ball(&frame.players, &frame.ball) {
            let dist = Self::distance(
                (closest_player.x, closest_player.y),
                (frame.ball.x, frame.ball.y),
            );
            // If within inner_rad, that player takes possession => new event
            if dist < self.inner_rad {
                let mut event = DribbleEvent::new(closest_player.id.clone(), frame.frame_number);

                // Possibly also detect defenders
                let defenders = self.detect_defenders(&frame.players, closest_player);
                for d in defenders {
                    event.add_defender(d.id.clone());
                }
                self.active_event = Some(event.clone());
                println!("Started new event: {:?}", event);
                return Some(event);
            }
        }
        None
    }

    /// Update or end the existing event.
    /// We do NOT change possession mid-event. If the ball leaves `inner_rad`,
    /// the event ends. We do not pass the ball to another player in here.
    /// If the event ends, return Some(...). Otherwise, return None.
    fn handle_active_event(
        &mut self,
        frame: &DribleFrame,
        event: &mut DribbleEvent,
    ) -> Option<DribbleEvent> {
        // The possession holder is fixed throughout the event
        let holder_id = event.possession_holder.clone();

        // Find the event's holder among the current frame's players
        if let Some(holder) = frame.players.iter().find(|p| p.id == holder_id) {
            // Check distance to the ball
            let dist = Self::distance((holder.x, holder.y), (frame.ball.x, frame.ball.y));
            
            if dist < self.inner_rad {
                // The ball is still in holder's range => update ongoing event
                event.add_frame(frame.frame_number);

                let defenders = self.detect_defenders(&frame.players, holder);
                for def in defenders {
                    event.add_defender(def.id.clone());
                }

                // If no defenders remain, we consider that a natural end
                if event.active_defenders.is_empty() {
                    event.end_frame = Some(frame.frame_number);
                    event.finished = true;
                    return self.active_event.take();
                }
                return self.active_event.clone();
            } else {
                // The ball left the holder's inner_rad => end event
                event.end_frame = Some(frame.frame_number);
                event.finished = true; // or false if you consider it 'cut short'
                return self.active_event.take();
            }
        } else {
            // The holder is not even in the frame (maybe they left?), end event
            event.end_frame = Some(frame.frame_number);
            event.finished = false;
            return self.active_event.take();
        }
    }
}
