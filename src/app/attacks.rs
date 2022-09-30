use super::*;

impl State {
    pub fn process_attacks(&mut self, delta_time: f32) {
        if !self.process_battle {
            self.feed = None;
            return;
        }
        if let Some(time) = &mut self.next_attack {
            *time -= delta_time * 3.0;
            if *time <= 0.0 {
                for attack in self.attacks.drain(..) {
                    if let Some(guy) = self.guys.get_mut(&attack.target_id) {
                        guy.health -= 1;
                    }
                }
                for guy in &self.guys {
                    if guy.health == 0 {
                        self.feed = Some(format!("{} has been eliminated", guy.name));

                        self.effects.push(Effect {
                            pos: guy.position,
                            scale_up: 1.0,
                            offset: 0.3,
                            size: 1.0,
                            time: 0.0,
                            max_time: 0.7,
                            back_texture: None,
                            front_texture: Some(self.assets.skull.clone()),
                            guy_id: Some(guy.id),
                            color: Rgba::BLACK,
                        });

                        let mut sound_effect = self.assets.death_sfx.effect();
                        sound_effect.set_volume(self.volume * 0.5);
                        sound_effect.play();
                    }
                }
                self.guys.retain(|guy| guy.health > 0);
                self.next_attack = None;
            }
        }
        if self.next_attack.is_some() {
            return;
        }

        let guys: Vec<&Guy> = self.guys.iter().collect();

        'schedule_attacks: loop {
            let new_attack = if let Some(attack) = self.queued_attack.take() {
                attack
            } else {
                let mut healths = HashMap::new();
                for guy in &self.guys {
                    healths.insert(guy.id, guy.health);
                }
                for attack in &self.attacks {
                    if let Some(health) = healths.get_mut(&attack.target_id) {
                        *health -= 1;
                    }
                }

                if self.guys.iter().any(|guy| !guy.should_never_win) {
                    if !self
                        .guys
                        .iter()
                        .any(|guy| !guy.should_never_win && healths[&guy.id] != 0)
                    {
                        self.attacks.clear();
                        break 'schedule_attacks;
                    }
                }

                if healths.values().filter(|health| **health == 0).count() != 0 {
                    break 'schedule_attacks;
                }

                let target = if let Ok(target) =
                    guys.choose_weighted(&mut global_rng(), |guy| healths[&guy.id])
                {
                    target
                } else {
                    break 'schedule_attacks;
                };
                let attacker = if let Some(attacker) = guys
                    .iter()
                    .copied()
                    .filter(|guy| guy.id != target.id && healths[&guy.id] != 0)
                    .min_by_key(|guy| r32((guy.position - target.position).len()))
                {
                    attacker
                } else {
                    break 'schedule_attacks;
                };
                Attack {
                    attacker_id: attacker.id,
                    target_id: target.id,
                }
            };
            if self
                .attacks
                .iter()
                .any(|current_attack| current_attack.attacker_id == new_attack.attacker_id)
            {
                println!("Queued {:?}", new_attack);
                self.queued_attack = Some(new_attack);
                break;
            } else {
                println!("Doing {:?}", new_attack);
                self.attacks.push(new_attack);
            }
        }
        self.next_attack = Some(1.0);
    }
}
