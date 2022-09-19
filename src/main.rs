use geng::prelude::*;

#[derive(HasId)]
struct Guy {
    id: i32,
    velocity: Vec2<f32>,
    position: Vec2<f32>,
}

struct Test {
    geng: Geng,
    guys: Collection<Guy>,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<usize>,
    next_id: i32,
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
        }
    }
}

impl geng::State for Test {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Rgba::BLACK), None, None);
        for guy in &self.guys {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::Ellipse::circle(guy.position, Test::GUY_RADIUS, Rgba::WHITE),
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
                        let id = self.next_id;
                        self.next_id += 1;
                        self.guys.insert(Guy {
                            id,
                            position,
                            velocity: Vec2::ZERO,
                        });
                    }
                    geng::MouseButton::Right => {
                        self.guys
                            .retain(|guy| (guy.position - position).len() > Test::GUY_RADIUS);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
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
}

fn main() {
    let geng = Geng::new("ttv");
    let geng = &geng;
    geng::run(geng, Test::new(geng));
}
