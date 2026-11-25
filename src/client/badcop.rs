use super::*;

#[derive(geng::asset::Load)]
struct Assets {
    badcop: ugli::Texture,
    #[load(path = "spin.wav")]
    spin_sfx: geng::Sound,
}

const SPINS_PER_SEC: f32 = 2.0;
const SPIN_TIME: f32 = 1.0 / SPINS_PER_SEC;
const FLY_TIME: f32 = 3.0 * SPIN_TIME;

struct Badcop {
    t: f32,
    next_spin: f32,
    pos: vec2<f32>,
    vel: vec2<f32>,
}

pub struct State {
    geng: Geng,
    assets: Assets,
    badcops: Vec<Badcop>,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            assets: geng::asset::Load::load(geng.asset_manager(), &path, &default())
                .await
                .unwrap(),
            geng,
            badcops: Vec::new(),
        }
    }

    async fn update(&mut self, delta_time: f32) {
        for badcop in &mut self.badcops {
            badcop.t += delta_time;
            if badcop.t < FLY_TIME && badcop.t > badcop.next_spin {
                self.assets
                    .spin_sfx
                    .play()
                    .set_speed(thread_rng().gen_range(0.9..1.1));
                badcop.next_spin += SPIN_TIME;
            }
            badcop.pos += badcop.vel * delta_time;
        }
        self.badcops.retain(|badcop| badcop.t < FLY_TIME);
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        for badcop in &self.badcops {
            self.geng.draw2d().draw2d(
                framebuffer,
                &geng::Camera2d {
                    center: vec2::ZERO,
                    rotation: Angle::ZERO,
                    fov: 10.0,
                },
                &draw2d::TexturedQuad::new(
                    Aabb2::point(vec2::ZERO).extend_uniform(1.0),
                    &self.assets.badcop,
                )
                .scale(vec2(1.0, (badcop.t * f32::PI * 2.0 * SPINS_PER_SEC).sin()))
                .rotate(badcop.vel.arg())
                .translate(badcop.pos),
            );
        }
    }

    async fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::RewardRedemption { name, reward } = message else {
            return;
        };
        if reward != "badcop 3 spin" {
            return;
        }
        let mut pos = vec2(thread_rng().gen_range(-5.0..5.0), -5.0);
        let mut target_pos = vec2(thread_rng().gen_range(-5.0..5.0), 5.0);
        if thread_rng().gen_bool(0.5) {
            mem::swap(&mut pos, &mut target_pos);
        }
        let vel = (target_pos - pos) / FLY_TIME;
        self.badcops.push(Badcop {
            t: 0.0,
            next_spin: 0.0,
            pos,
            vel,
        });
    }
}
