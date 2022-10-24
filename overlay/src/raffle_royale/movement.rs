use super::*;

impl RaffleRoyale {
    pub fn process_movement(&mut self, delta_time: f32) {
        let Circle { center, .. } = match self.find_circle() {
            Some(circle) => circle,
            None => return,
        };
        let ids = self.guys.ids().copied().collect::<Vec<_>>();

        // Guys do be accelerating
        for id in &ids {
            let mut guy = self.guys.remove(id).unwrap();
            let target_velocity =
                (center - guy.position).normalize_or_zero() * RaffleRoyale::GUY_MAX_SPEED;
            guy.velocity += (target_velocity - guy.velocity)
                .clamp_len(..=RaffleRoyale::GUY_ACCELERATION * delta_time);
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
                    if len < RaffleRoyale::MIN_DISTANCE {
                        let v = delta_pos.normalize_or_zero();
                        moves.push((guy.id, v * (RaffleRoyale::MIN_DISTANCE - len) / 2.0));
                        guy.velocity -= v * Vec2::dot(guy.velocity, v);
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
