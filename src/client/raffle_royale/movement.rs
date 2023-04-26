use super::*;

impl State {
    pub fn process_movement(&mut self, delta_time: f32) {
        let Circle { center, .. } = match self.find_circle() {
            Some(circle) => circle,
            None => return,
        };
        let ids = self.guys.ids().copied().collect::<Vec<_>>();

        // Guys do be accelerating
        for id in &ids {
            use noise::NoiseFn;
            let mut guy = self.guys.remove(id).unwrap();
            let mut target_velocity =
                (center - guy.position).normalize_or_zero() * State::GUY_MAX_SPEED;
            target_velocity += target_velocity.rotate_90()
                * self.noise.get([guy.id as f64, self.time as f64]) as f32;
            guy.velocity +=
                (target_velocity - guy.velocity).clamp_len(..=State::GUY_ACCELERATION * delta_time);
            self.guys.insert(guy);
        }

        // Guys do be moving
        for guy in &mut self.guys {
            guy.position += guy.velocity * delta_time;
        }
        // Guys do be colliding
        for _ in 0..10 {
            let mut moves = Vec::new();
            for id in &ids {
                let mut guy = self.guys.remove(id).unwrap();
                for other in &self.guys {
                    let delta_pos = guy.position - other.position;
                    let len = delta_pos.len();
                    let v = delta_pos.normalize_or_zero();
                    if len < State::MIN_DISTANCE {
                        moves.push((guy.id, v * (State::MIN_DISTANCE - len) / 2.0));
                        guy.velocity -= v * vec2::dot(guy.velocity, v);
                    } else if len < State::PREFERRED_DISTANCE {
                        guy.velocity += v
                            * (State::PREFERRED_DISTANCE - len)
                            * self.assets.constants.preferred_distance_spring_k;
                    }
                }
                self.guys.insert(guy);
            }
            for (id, v) in moves {
                let mut guy = self.guys.remove(&id).unwrap();
                guy.position += v;
                self.guys.insert(guy);
            }
        }
    }
}
