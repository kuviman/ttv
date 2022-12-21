use super::*;

#[derive(Deserialize, geng::Assets)]
#[asset(json)]
struct Config {
    fart_color: Rgba<f32>,
}

#[derive(geng::Assets)]
struct Assets {
    #[asset(load_with = "load_custom(&geng, base_path.join(\"custom\"))")]
    custom: HashMap<String, ugli::Texture>,
    config: Config,
    #[asset(postprocess = "pixelate")]
    crab: ugli::Texture,
    #[asset(range = "1..=3", path = "fart*.wav")]
    fart: Vec<geng::Sound>,
    farticle: ugli::Texture,
}

async fn load_custom(
    geng: &Geng,
    path: std::path::PathBuf,
) -> anyhow::Result<HashMap<String, ugli::Texture>> {
    let path = &path;
    let json: String = geng::LoadAsset::load(geng, &path.join("list.json")).await?;
    let names: Vec<String> = serde_json::from_str(&json).unwrap();
    Ok(future::join_all(names.into_iter().map(|name| async move {
        let texture = geng::LoadAsset::load(geng, &path.join(format!("{name}.png")))
            .await
            .unwrap();
        (name, texture)
    }))
    .await
    .into_iter()
    .collect())
}

struct Crab {
    t: f32,
    pos: Vec2<f32>,
    vel: Vec2<f32>,
    target_pos: f32,
    stand_timer: f32,
    text: Option<String>,
    text_timer: f32,
    farts: usize,
    fart_timer: f32,
    custom: Option<String>,
}

struct Farticle {
    size: f32,
    pos: Vec2<f32>,
    vel: Vec2<f32>,
    color: Rgba<f32>,
    rot: f32,
    w: f32,
    t: f32,
}

const CRAB_ACCELERATION: f32 = 10.0;
const CRAB_SPEED: f32 = 1.0;
const GRAVITY: f32 = 10.0;

impl Crab {
    fn new() -> Self {
        Self {
            t: 0.0,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            target_pos: 0.0,
            stand_timer: 0.0,
            text: None,
            text_timer: 0.0,
            farts: 0,
            fart_timer: 0.0,
            custom: None,
        }
    }
    fn update(&mut self, screen_size: Vec2<f32>, delta_time: f32) {
        self.t += delta_time;
        self.vel.y -= GRAVITY * delta_time;
        self.vel.x += ((self.target_pos - self.pos.x).clamp_abs(CRAB_SPEED) - self.vel.x)
            .clamp_abs(CRAB_ACCELERATION * delta_time);
        self.pos += self.vel * delta_time;
        if self.pos.y < -screen_size.y + 1.0 {
            self.pos.y = -screen_size.y + 1.0;
            self.vel.y = self.vel.y.max(0.0);
        }
        if (self.pos.x - self.target_pos).abs() < 0.1 {
            self.stand_timer -= delta_time;
            if self.stand_timer < 0.0 {
                self.stand_timer = global_rng().gen_range(1.0..=5.0);
                self.target_pos =
                    global_rng().gen_range(-screen_size.x + 1.0..=screen_size.x - 1.0);
            }
        }

        if self.text.is_some() {
            self.text_timer -= delta_time;
            if self.text_timer < 0.0 {
                self.text = None;
            }
        }
    }
}

pub struct State {
    geng: Geng,
    assets: Assets,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<f32>,
    time: f32,
    crabs: HashMap<String, Crab>,
    farticles: Vec<Farticle>,
    connection: Connection,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            connection,
            assets: geng::LoadAsset::load(&geng, &path).await.unwrap(),
            geng,
            framebuffer_size: vec2(1.0, 1.0),
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 20.0,
            },
            time: 0.0,
            crabs: default(),
            farticles: vec![],
        }
    }
    async fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
        let screen_size =
            vec2(self.framebuffer_size.x / self.framebuffer_size.y, 1.0) * self.camera.fov / 2.0;
        for crab in self.crabs.values_mut() {
            crab.update(screen_size, delta_time);

            if crab.farts != 0 {
                crab.fart_timer -= delta_time;
                if crab.fart_timer < 0.0 {
                    crab.fart_timer = 0.5;
                    crab.farts -= 1;
                    self.assets.fart.choose(&mut global_rng()).unwrap().play();
                    for _ in 0..20 {
                        self.farticles.push(Farticle {
                            size: 0.5,
                            pos: crab.pos
                                + vec2(
                                    global_rng().gen_range(-1.0..1.0),
                                    global_rng().gen_range(-1.0..1.0),
                                ) * 0.5,
                            vel: crab.vel
                                + vec2(
                                    global_rng().gen_range(-1.0..1.0),
                                    global_rng().gen_range(-1.0..1.0),
                                ) * 0.5,
                            color: self.assets.config.fart_color,
                            rot: global_rng().gen_range(0.0..2.0 * f32::PI),
                            w: global_rng().gen_range(-1.0..1.0) * 3.0,
                            t: 1.0,
                        });
                    }
                    crab.vel.y += 10.0;
                }
            }
        }

        for farticle in &mut self.farticles {
            farticle.t -= delta_time;
            farticle.pos += farticle.vel * delta_time;
            farticle.rot += farticle.w * delta_time;
        }
        self.farticles.retain(|farticle| farticle.t > 0.0);
    }
    async fn handle_event(&mut self, event: geng::Event) {}
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        let font: &geng::Font = self.geng.default_font();
        for (name, crab) in &self.crabs {
            let y = (crab.t * 10.0).sin();
            let mov = (crab.vel.len() / CRAB_SPEED).min(1.0);
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit(
                    crab.custom
                        .as_deref()
                        .and_then(|name| self.assets.custom.get(name))
                        .unwrap_or(&self.assets.crab),
                )
                .transform(Mat3::rotate(y * 0.1 * mov))
                .translate(crab.pos + vec2(0.0, y.abs() * 0.1 * mov)),
            );
            font.draw_with_outline(
                framebuffer,
                &self.camera,
                name,
                crab.pos + vec2(0.0, 1.0),
                geng::TextAlign::CENTER,
                0.5,
                Rgba::BLACK,
                0.04,
                Rgba::WHITE,
            );

            if let Some(text) = &crab.text {
                let mut lines = Vec::new();
                let mut line = String::new();
                for word in text.split_whitespace() {
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(word);
                    if line.len() > 30 {
                        lines.push(mem::replace(&mut line, String::new()));
                    }
                }
                if !line.is_empty() {
                    lines.push(line);
                }
                const MAX_LINES: usize = 5;
                if lines.len() > MAX_LINES {
                    lines.truncate(MAX_LINES);
                    lines.last_mut().unwrap().push_str(" ...");
                }
                let start_y = 2.0;
                let mut y = start_y;
                for line in lines.iter().rev() {
                    if let Some(aabb) = font.measure(line, 0.5) {
                        self.geng.draw_2d(
                            framebuffer,
                            &self.camera,
                            &draw_2d::Quad::new(
                                aabb.extend_uniform(0.4)
                                    .translate(crab.pos + vec2(-aabb.width() / 2.0, y)),
                                Rgba::BLACK,
                            ),
                        );
                    }
                    y += 0.5;
                }
                y = start_y;
                for line in lines.iter().rev() {
                    if let Some(aabb) = font.measure(line, 0.5) {
                        self.geng.draw_2d(
                            framebuffer,
                            &self.camera,
                            &draw_2d::Quad::new(
                                aabb.extend_uniform(0.2)
                                    .translate(crab.pos + vec2(-aabb.width() / 2.0, y)),
                                Rgba::WHITE,
                            ),
                        );
                    }
                    y += 0.5;
                }
                y = start_y;
                for line in lines.iter().rev() {
                    font.draw(
                        framebuffer,
                        &self.camera,
                        line,
                        crab.pos + vec2(0.0, y),
                        geng::TextAlign::CENTER,
                        0.5,
                        Rgba::BLACK,
                    );
                    y += 0.5;
                }
            }
        }
        for farticle in &self.farticles {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(
                    &self.assets.farticle,
                    Rgba {
                        a: farticle.color.a * farticle.t,
                        ..farticle.color
                    },
                )
                .transform(Mat3::rotate(farticle.rot))
                .scale_uniform(farticle.size)
                .translate(farticle.pos),
            )
        }
    }
    async fn handle(&mut self, message: &ServerMessage) {
        if let ServerMessage::ChatMessage { name, message } = message {
            let parts: Vec<&str> = message.split_whitespace().collect();
            if name == "kuviman" && parts.first() == Some(&"!setavatar") && parts.len() == 3 {
                self.connection
                    .set_key_value(&format!("avatars/{}", parts[1]), &parts[2]);
                return;
            }
            let crab = self.crabs.entry(name.to_owned()).or_insert_with(Crab::new);
            crab.custom = self
                .connection
                .get_key_value(&format!("avatars/{name}"))
                .await;
            match message.trim() {
                "!jump" => {
                    crab.vel.y += 10.0;
                }
                "!fart" => {
                    crab.farts = 1;
                    crab.fart_timer = 0.0;
                }
                "!doublefart" => {
                    crab.farts = 2;
                    crab.fart_timer = 0.0;
                }
                _ => {
                    crab.text = Some(message.to_owned());
                    crab.text_timer = 5.0;
                }
            }
        }
    }
}
