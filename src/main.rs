use geng::prelude::*;

type Id = i32;

#[derive(HasId)]
struct Guy {
    id: Id,
    health: usize,
    velocity: Vec2<f32>,
    position: Vec2<f32>,
}

struct Attack {
    attacker_id: Id,
    target_id: Id,
}

struct Test {
    geng: Geng,
    guys: Collection<Guy>,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<usize>,
    next_id: Id,
    process_battle: bool,
    next_attack: Option<f32>,
    attack: Option<Attack>,
}

impl Test {
    const GUY_RADIUS: f32 = 1.0;
    const MIN_DISTANCE: f32 = 5.0;
    const GUY_MAX_SPEED: f32 = 10.0;
    const GUY_ACCELERATION: f32 = 10.0;
    pub fn new(geng: &Geng) -> Self {
        Self {
            next_id: 0,
            geng: geng.clone(),
            guys: default(),
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 50.0,
            },
            framebuffer_size: vec2(1, 1),
            process_battle: false,
            next_attack: None,
            attack: None,
        }
    }

    fn process_movement(&mut self, delta_time: f32) {
        let ids = self.guys.ids().copied().collect::<Vec<_>>();
        let center = if self.guys.is_empty() {
            None
        } else {
            let mut sum = Vec2::ZERO;
            for guy in &self.guys {
                sum += guy.position;
            }
            Some(sum / self.guys.len() as f32)
        };
        // Guys do be accelerating
        for id in &ids {
            let mut guy = self.guys.remove(id).unwrap();
            if let Some(center) = center {
                let target_velocity =
                    (center - guy.position).normalize_or_zero() * Test::GUY_MAX_SPEED;
                guy.velocity += (target_velocity - guy.velocity)
                    .clamp_len(..=Test::GUY_ACCELERATION * delta_time);
            }
            self.guys.insert(guy);
        }
        // Guys do be moving
        for guy in &mut self.guys {
            guy.position += guy.velocity * delta_time;
        }
        let mut moves = Vec::new();
        for id in &ids {
            let mut guy = self.guys.remove(id).unwrap();
            for other in &self.guys {
                let delta_pos = guy.position - other.position;
                let len = delta_pos.len();
                if len < Test::MIN_DISTANCE {
                    let v = delta_pos.normalize_or_zero();
                    moves.push((guy.id, v * (Test::MIN_DISTANCE - len) / 2.0));
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

    fn process_attacks(&mut self, delta_time: f32) {
        if !self.process_battle {
            return;
        }
        if let Some(time) = &mut self.next_attack {
            *time -= delta_time;
            if *time <= 0.0 {
                let attack = self.attack.take().unwrap();
                self.guys.get_mut(&attack.target_id).unwrap().health -= 1;
                self.guys.retain(|guy| guy.health > 0);
                self.next_attack = None;
            }
        }
        if self.next_attack.is_some() {
            return;
        }
        if self.guys.len() < 2 {
            return;
        }

        let guys: Vec<&Guy> = self.guys.iter().collect();
        let target = guys
            .choose_weighted(&mut global_rng(), |guy| guy.health)
            .unwrap();
        let target_id = target.id;

        let attacker = guys
            .iter()
            .copied()
            .filter(|guy| guy.id != target.id)
            .min_by_key(|guy| r32((guy.position - target.position).len()))
            .unwrap();
        self.attack = Some(Attack {
            attacker_id: attacker.id,
            target_id: target.id,
        });
        self.next_attack = Some(1.0);
    }
}

impl geng::State for Test {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
        if let Some(attack) = &self.attack {
            let attacker = self.guys.get(&attack.attacker_id).unwrap();
            let target = self.guys.get(&attack.target_id).unwrap();
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::Segment::new_gradient(
                    draw_2d::ColoredVertex {
                        a_pos: attacker.position,
                        a_color: Rgba::WHITE,
                    },
                    draw_2d::ColoredVertex {
                        a_pos: target.position,
                        a_color: Rgba::new(1.0, 0.5, 0.0, 1.0),
                    },
                    Test::GUY_RADIUS * 0.2,
                ),
            )
        }
        for guy in &self.guys {
            let mut color = Rgba::WHITE;
            if let Some(attack) = &self.attack {
                if guy.id == attack.attacker_id {
                    color = Rgba::YELLOW;
                }
                if guy.id == attack.target_id {
                    color = Rgba::RED;
                }
            }
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::Ellipse::circle(guy.position, Test::GUY_RADIUS, color),
            );
            self.geng.default_font().draw(
                framebuffer,
                &self.camera,
                &format!("{}", guy.health),
                guy.position + vec2(0.0, Self::GUY_RADIUS * 1.1),
                geng::TextAlign::CENTER,
                Self::GUY_RADIUS,
                Rgba::GREEN,
            );
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MouseDown { position, button } => {
                let position = self.camera.screen_to_world(
                    self.framebuffer_size.map(|x| x as f32),
                    position.map(|x| x as f32),
                );
                match button {
                    geng::MouseButton::Left => {
                        let mut iter = self.guys.iter_mut();
                        if let Some(guy) =
                            iter.find(|guy| (guy.position - position).len() < Test::GUY_RADIUS)
                        {
                            guy.health += 1;
                        } else {
                            let id = self.next_id;
                            self.next_id += 1;
                            mem::drop(iter);
                            self.guys.insert(Guy {
                                id,
                                position,
                                velocity: Vec2::ZERO,
                                health: 1,
                            });
                        }
                    }
                    geng::MouseButton::Right => {
                        self.guys
                            .retain(|guy| (guy.position - position).len() > Test::GUY_RADIUS);
                    }
                    _ => {}
                }
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::Space => {
                    self.process_battle = !self.process_battle;
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        self.process_movement(delta_time);
        self.process_attacks(delta_time);
    }
}

fn main() {
    let geng = Geng::new("ttv");
    let geng = &geng;
    geng::run(geng, Test::new(geng));
}
