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

    pub fn detect_defenders<'a>(
        &self,
        players: &'a [Player],
        possession_holder: &Player,
    ) -> Vec<&'a Player> {
        players
            .iter()
            .filter(|player: &&Player| {
                player.id != possession_holder.id
                    && Self::distance(
                        (player.x, player.y),
                        (possession_holder.x, possession_holder.y),
                    ) < self.outer_rad
            })
            .collect()
    }

    pub fn process_frame(&mut self, frame: DribleFrame) -> Option<DribbleEvent> {
        if let Some(event) = &mut self.active_event {
            let mut event_clone = event.clone();
            println!("Active drible event");
            self.handle_active_event(frame, &mut event_clone)
        } else {
            println!("No active event");
            self.handle_search_state(frame)
        }
    }

    pub fn handle_search_state(&mut self, frame: DribleFrame) -> Option<DribbleEvent> {
        if let Some(possession_holder) = self.closest_player_to_ball(&frame.players, &frame.ball) {
            let defenders = self.detect_defenders(&frame.players, possession_holder);

            if !defenders.is_empty() {
                let mut event = DribbleEvent::new(possession_holder.id, frame.frame_number);
                for defender in defenders {
                    event.add_defender(defender.id);
                }
                self.active_event = Some(event);
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn handle_active_event(
        &mut self,
        frame: DribleFrame,
        event: &mut DribbleEvent,
    ) -> Option<DribbleEvent> {
        if let Some(possession_holder) = self.closest_player_to_ball(&frame.players, &frame.ball) {
            if possession_holder.id == event.possession_holder {
                event.add_frame(frame.frame_number);

                let defenders = self.detect_defenders(&frame.players, possession_holder);
                for defender in defenders {
                    event.add_defender(defender.id);
                }

                None
            } else {
                event.end_frame = Some(frame.frame_number);
                let completed_event = self.active_event.take();
                completed_event
            }
        } else {
            self.active_event = None;
            None
        }
    }
}
