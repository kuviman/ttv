use super::*;

#[derive(Deserialize, geng::asset::Load)]
#[load(serde = "json")]
struct Config {
    fart_color: Rgba<f32>,
}

#[derive(geng::asset::Load)]
struct Assets {
    #[load(load_with = "load_custom(&manager, base_path.join(\"custom\"))")]
    custom: HashMap<String, ugli::Texture>,
    config: Config,
    #[load(postprocess = "pixelate")]
    crab: ugli::Texture,
    #[load(list = "1..=3", path = "fart*.wav")]
    fart: Vec<geng::Sound>,
    farticle: ugli::Texture,
}

async fn load_custom(
    manager: &geng::asset::Manager,
    path: std::path::PathBuf,
) -> anyhow::Result<HashMap<String, ugli::Texture>> {
    let path = &path;
    let json: String =
        geng::asset::Load::load(manager, &path.join("list.json"), &default()).await?;
    let names: Vec<String> = serde_json::from_str(&json).unwrap();
    Ok(future::join_all(names.into_iter().map(|name| async move {
        let texture =
            geng::asset::Load::load(manager, &path.join(format!("{name}.png")), &default())
                .await
                .unwrap();
        (name, texture)
    }))
    .await
    .into_iter()
    .collect())
}

struct Crab {
    ground: bool,
    t: f32,
    pos: vec2<f32>,
    vel: vec2<f32>,
    target_pos: f32,
    stand_timer: f32,
    text: Option<String>,
    text_timer: f32,
    farts: usize,
    fart_timer: f32,
    custom: Option<String>,
    score: i32,
}

struct Farticle {
    size: f32,
    pos: vec2<f32>,
    vel: vec2<f32>,
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
            score: 0,
            ground: false,
            t: 0.0,
            pos: vec2(0.0, -5.0),
            vel: vec2::ZERO,
            target_pos: 0.0,
            stand_timer: 0.0,
            text: None,
            text_timer: 0.0,
            farts: 0,
            fart_timer: 0.0,
            custom: None,
        }
    }
    fn update(&mut self, screen_size: vec2<f32>, delta_time: f32) {
        self.t += delta_time;
        self.vel.y -= GRAVITY * delta_time;
        if self.ground {
            self.vel.x += ((self.target_pos - self.pos.x).clamp_abs(CRAB_SPEED) - self.vel.x)
                .clamp_abs(CRAB_ACCELERATION * delta_time);
        }
        self.pos += self.vel * delta_time;
        self.ground = false;
        if self.pos.y < -screen_size.y + 1.0 {
            self.ground = true;
            self.pos.y = -screen_size.y + 1.0;
            self.vel.y = self.vel.y.max(0.0);
        }

        if self.pos.y > screen_size.y - 1.0 && self.vel.y > 0.0 {
            self.ground = true;
            self.pos.y = screen_size.y - 1.0;
            self.vel.y = -self.vel.y;
        }
        if self.pos.x.abs() > screen_size.x - 1.0 {
            self.ground = true;
            self.pos.x = (screen_size.x - 1.0) * self.pos.x.signum();
            self.vel.x = -self.vel.x;
        }

        if (self.pos.x - self.target_pos).abs() < 0.1 {
            self.stand_timer -= delta_time;
            if self.stand_timer < 0.0 {
                self.stand_timer = thread_rng().gen_range(1.0..=5.0);
                self.target_pos =
                    thread_rng().gen_range(-screen_size.x + 1.0..=screen_size.x - 1.0);
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
    disabled: bool,
    geng: Geng,
    assets: Assets,
    camera: geng::Camera2d,
    framebuffer_size: vec2<f32>,
    time: f32,
    crabs: HashMap<String, Crab>,
    farticles: Vec<Farticle>,
    connection: Connection,
    bounce_points: Vec<vec2<f32>>,
    bouncy_game: bool,
    breakout: bool,
    bricks: Vec<Aabb2<f32>>,
    win_timer: f32,
    grid_size: i32,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            disabled: false,
            geng: geng.clone(),
            grid_size: 10,
            win_timer: 0.0,
            breakout: false,
            bricks: Vec::new(),
            bouncy_game: false,
            bounce_points: {
                let mut v = Vec::new();
                for x in -5..=5 {
                    for y in -2..=2 {
                        if (x + y) % 2 == 0 {
                            continue;
                        }
                        v.push(vec2(x as f32, y as f32) * 2.5);
                    }
                }
                v
            },
            connection,
            assets: geng::asset::Load::load(geng.asset_manager(), &path, &default())
                .await
                .unwrap(),
            framebuffer_size: vec2(1.0, 1.0),
            camera: geng::Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 30.0,
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

        if self.breakout && self.bricks.is_empty() {
            self.win_timer -= delta_time;
            if self.win_timer < 0.0 {
                self.win_timer = 20.0;
                let brick_size = vec2(
                    screen_size.x / self.grid_size as f32,
                    screen_size.y / self.grid_size as f32,
                );
                for x in -self.grid_size..self.grid_size {
                    for y in -self.grid_size..self.grid_size {
                        self.bricks.push(
                            Aabb2::point(vec2::ZERO)
                                .extend_positive(brick_size)
                                .translate(vec2(x as f32, y as f32) * brick_size),
                        );
                    }
                }
            }
        }

        for crab in self.crabs.values_mut() {
            crab.update(screen_size, delta_time);

            if self.bouncy_game {
                for &p in &self.bounce_points {
                    let delta_pos = crab.pos - p;
                    let pen = 1.0 + 0.2 - delta_pos.len();
                    if pen > 0.0 {
                        let n = delta_pos.normalize_or_zero();
                        crab.pos += n * pen;
                        crab.vel -= n * vec2::dot(n, crab.vel);
                        crab.vel += n * 10.0;
                    }
                }
            }

            if self.breakout {
                let mut destroyed = Vec::new();
                for (i, &p) in self.bricks.iter().enumerate() {
                    let dx = if crab.pos.x < p.min.x {
                        crab.pos.x - p.min.x
                    } else if crab.pos.x > p.max.x {
                        crab.pos.x - p.max.x
                    } else {
                        0.0
                    };
                    let dy = if crab.pos.y < p.min.y {
                        crab.pos.y - p.min.y
                    } else if crab.pos.y > p.max.y {
                        crab.pos.y - p.max.y
                    } else {
                        0.0
                    };
                    let delta_pos = vec2(dx, dy);
                    let pen = 1.0 - delta_pos.len();
                    if pen > 0.0 {
                        let n = delta_pos.normalize_or_zero();
                        crab.pos += n * pen;
                        crab.vel -= n * vec2::dot(n, crab.vel);
                        crab.vel += n * 10.0;
                        destroyed.push(i);
                        crab.score += 1;
                    }
                }
                for index in destroyed.into_iter().rev() {
                    self.bricks.remove(index);
                }
            }

            if crab.farts != 0 {
                crab.fart_timer -= delta_time;
                if crab.fart_timer < 0.0 {
                    crab.fart_timer = 0.5;
                    crab.farts -= 1;
                    self.assets.fart.choose(&mut thread_rng()).unwrap().play();
                    for _ in 0..20 {
                        self.farticles.push(Farticle {
                            size: 0.5,
                            pos: crab.pos
                                + vec2(
                                    thread_rng().gen_range(-1.0..1.0),
                                    thread_rng().gen_range(-1.0..1.0),
                                ) * 0.5,
                            vel: crab.vel
                                + vec2(
                                    thread_rng().gen_range(-1.0..1.0),
                                    thread_rng().gen_range(-1.0..1.0),
                                ) * 0.5,
                            color: self.assets.config.fart_color,
                            rot: thread_rng().gen_range(0.0..2.0 * f32::PI),
                            w: thread_rng().gen_range(-1.0..1.0) * 3.0,
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
        if self.disabled {
            return;
        }
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        let font: &geng::Font = self.geng.default_font();
        if self.bouncy_game {
            ugli::clear(framebuffer, Some(Rgba::new(0.0, 0.0, 0.0, 0.5)), None, None);
            font.draw_with_outline(
                framebuffer,
                &self.camera,
                "!drop",
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(vec2(0.0, 8.5)),
                Rgba::WHITE,
                0.05,
                Rgba::BLACK,
            );
        }
        for (name, crab) in &self.crabs {
            let y = (crab.t * 10.0).sin();
            let mov = (crab.vel.len() / CRAB_SPEED).min(1.0);
            self.geng.draw2d().draw2d(
                framebuffer,
                &self.camera,
                &draw2d::TexturedQuad::unit(
                    crab.custom
                        .as_deref()
                        .and_then(|name| self.assets.custom.get(name))
                        .unwrap_or(&self.assets.crab),
                )
                .transform(mat3::rotate(Angle::from_radians(y * 0.1 * mov)))
                .translate(crab.pos + vec2(0.0, y.abs() * 0.1 * mov)),
            );
            font.draw_with_outline(
                framebuffer,
                &self.camera,
                name,
                vec2::splat(geng::TextAlign::CENTER),
                mat3::translate(crab.pos + vec2(0.0, 1.0)) * mat3::scale_uniform(0.5),
                Rgba::BLACK,
                0.08,
                Rgba::WHITE,
            );
            if self.breakout {
                font.draw_with_outline(
                    framebuffer,
                    &self.camera,
                    &format!("score: {}", crab.score),
                    vec2::splat(geng::TextAlign::CENTER),
                    mat3::translate(crab.pos + vec2(0.0, 1.5)) * mat3::scale_uniform(0.5),
                    Rgba::BLACK,
                    0.08,
                    Rgba::WHITE,
                );
            }

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
                    if let Some(aabb) = font
                        .measure(line, vec2::splat(geng::TextAlign::LEFT))
                        .map(|bb| bb.map(|x| x * 0.5))
                    {
                        self.geng.draw2d().draw2d(
                            framebuffer,
                            &self.camera,
                            &draw2d::Quad::new(
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
                    if let Some(aabb) = font
                        .measure(line, vec2::splat(geng::TextAlign::LEFT))
                        .map(|bb| bb.map(|x| x * 0.5))
                    {
                        self.geng.draw2d().draw2d(
                            framebuffer,
                            &self.camera,
                            &draw2d::Quad::new(
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
                        vec2::splat(geng::TextAlign::CENTER),
                        mat3::translate(crab.pos + vec2(0.0, y)) * mat3::scale_uniform(0.5),
                        Rgba::BLACK,
                    );
                    y += 0.5;
                }
            }
        }
        for farticle in &self.farticles {
            self.geng.draw2d().draw2d(
                framebuffer,
                &self.camera,
                &draw2d::TexturedQuad::unit_colored(
                    &self.assets.farticle,
                    Rgba {
                        a: farticle.color.a * farticle.t,
                        ..farticle.color
                    },
                )
                .transform(mat3::rotate(Angle::from_radians(farticle.rot)))
                .scale_uniform(farticle.size)
                .translate(farticle.pos),
            )
        }
        if self.bouncy_game {
            for &point in &self.bounce_points {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Ellipse::circle(point, 0.3, Rgba::WHITE),
                );
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Ellipse::circle(point, 0.2, Rgba::BLACK),
                );
            }
        }
        if self.breakout {
            for &p in &self.bricks {
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Quad::new(p, Rgba::BLACK),
                );
                self.geng.draw2d().draw2d(
                    framebuffer,
                    &self.camera,
                    &draw2d::Quad::new(p.extend_uniform(-0.1), Rgba::WHITE),
                );
            }
            if self.bricks.is_empty() {
                if let Some((name, winner)) = self.crabs.iter().max_by_key(|(_, crab)| crab.score) {
                    font.draw_with_outline(
                        framebuffer,
                        &self.camera,
                        "winner is",
                        vec2::splat(geng::TextAlign::CENTER),
                        mat3::translate(vec2(0.0, 4.0)) * mat3::scale_uniform(2.0),
                        Rgba::BLACK,
                        0.04,
                        Rgba::WHITE,
                    );
                    font.draw_with_outline(
                        framebuffer,
                        &self.camera,
                        name,
                        vec2::splat(geng::TextAlign::CENTER),
                        mat3::translate(vec2(0.0, 0.0)) * mat3::scale_uniform(4.0),
                        Rgba::BLACK,
                        0.08,
                        Rgba::WHITE,
                    );
                    font.draw_with_outline(
                        framebuffer,
                        &self.camera,
                        &format!("with score of {}", winner.score),
                        vec2::splat(geng::TextAlign::CENTER),
                        mat3::translate(vec2(0.0, -2.0)) * mat3::scale_uniform(2.0),
                        Rgba::BLACK,
                        0.04,
                        Rgba::WHITE,
                    );
                }
            }
        }
    }
    async fn handle(&mut self, message: &ServerMessage) {
        if let ServerMessage::ChatMessage {
            id: _,
            name,
            message,
        } = message
        {
            let parts: Vec<&str> = message.split_whitespace().collect();
            if name == "kuviman" {
                if message == "!toggle avatars" {
                    self.disabled = !self.disabled;
                }
                if parts.first() == Some(&"!setavatar") && parts.len() == 3 {
                    self.connection
                        .set_key_value(&format!("avatars/{}", parts[1]), &parts[2]);
                    if let Some(crab) = self.crabs.get_mut(parts[1]) {
                        crab.custom = Some(parts[2].to_owned());
                    }
                    return;
                }
                if message == "!bounce" {
                    self.bouncy_game = !self.bouncy_game;
                    return;
                }
                if parts.first() == Some(&"!breakout") {
                    if let Some(size) = parts.get(1) {
                        if let Ok(size) = size.parse() {
                            self.grid_size = size;
                        }
                    }
                    self.breakout = !self.breakout;
                    self.bricks.clear();
                    self.win_timer = 0.0;
                    return;
                }
            }
            let crab = self.crabs.entry(name.to_owned()).or_insert_with(Crab::new);
            crab.custom = self
                .connection
                .get_key_value(&format!("avatars/{name}"))
                .await;
            if parts.first() == Some(&"!jump") {
                let angle = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                let angle = angle.clamp_abs(45);
                crab.vel = vec2(0.0, if self.breakout { 30.0 } else { 10.0 })
                    .rotate(Angle::from_radians(angle as f32 * f32::PI / 180.0));
                return;
            }
            match message.trim() {
                "!fart" => {
                    crab.farts = 1;
                    crab.fart_timer = 0.0;
                }
                "!doublefart" => {
                    crab.farts = 2;
                    crab.fart_timer = 0.0;
                }
                "!drop" if self.bouncy_game => {
                    crab.pos = vec2(
                        thread_rng().gen_range(-10.0..10.0),
                        self.camera.fov / 2.0 + 1.0,
                    );
                }
                _ => {
                    crab.text = Some(message.to_owned());
                    crab.text_timer = 5.0;
                }
            }
        }
    }
}
