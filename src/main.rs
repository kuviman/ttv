use geng::prelude::*;

#[derive(geng::Assets)]
pub struct Assets {
    pub hat: ugli::Texture,
    pub face: ugli::Texture,
    pub fireball: ugli::Texture,
}

type Id = i32;

#[derive(HasId)]
struct Guy {
    id: Id,
    health: usize,
    velocity: Vec2<f32>,
    position: Vec2<f32>,
}

#[derive(Debug)]
struct Attack {
    attacker_id: Id,
    target_id: Id,
}

struct Test {
    geng: Geng,
    assets: Rc<Assets>,
    guys: Collection<Guy>,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<usize>,
    next_id: Id,
    process_battle: bool,
    next_attack: Option<f32>,
    attacks: Vec<Attack>,
    queued_attack: Option<Attack>,
}

impl Test {
    const GUY_RADIUS: f32 = 1.0;
    const MIN_DISTANCE: f32 = 5.0;
    const GUY_MAX_SPEED: f32 = 10.0;
    const GUY_ACCELERATION: f32 = 10.0;
    pub fn new(geng: &Geng, assets: &Rc<Assets>) -> Self {
        Self {
            next_id: 0,
            geng: geng.clone(),
            assets: assets.clone(),
            guys: default(),
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 50.0,
            },
            framebuffer_size: vec2(1, 1),
            process_battle: false,
            next_attack: None,
            attacks: vec![],
            queued_attack: None,
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
            *time -= delta_time * 3.0;
            if *time <= 0.0 {
                for attack in self.attacks.drain(..) {
                    self.guys.get_mut(&attack.target_id).unwrap().health -= 1;
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
                    *healths.get_mut(&attack.target_id).unwrap() -= 1;
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

impl geng::State for Test {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some("#73882C".try_into().unwrap()), None, None);

        let t = 1.0 - self.next_attack.unwrap_or(0.0);
        for attack in &self.attacks {
            let attacker = self.guys.get(&attack.attacker_id).unwrap();
            let target = self.guys.get(&attack.target_id).unwrap();
            let v = target.position - attacker.position;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(vec2(0.0, 0.0)).extend_uniform(1.0),
                    &self.assets.fireball,
                )
                .transform(Mat3::rotate(v.arg()))
                .translate(attacker.position + v * t),
            );
        }

        for guy in &self.guys {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::new(
                    AABB::point(guy.position).extend_uniform(Test::GUY_RADIUS),
                    &self.assets.face,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::new(
                    AABB::point(guy.position).extend_uniform(Test::GUY_RADIUS),
                    &self.assets.hat,
                ),
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
                                health: 5,
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
    geng::run(
        geng,
        geng::LoadingScreen::new(
            geng,
            geng::EmptyLoadingScreen,
            <Assets as geng::LoadAsset>::load(geng, &static_path()),
            {
                let geng = geng.clone();
                move |assets| Test::new(&geng, &Rc::new(assets.unwrap()))
            },
        ),
    );
}
