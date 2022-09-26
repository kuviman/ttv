use super::*;

#[derive(Deserialize, geng::Assets)]
#[asset(json)]
pub struct Config {
    pub background: Rgba<f32>,
    pub circle: Rgba<f32>,
    pub initial_health: usize,
    pub health_increase_per_level: usize,
    pub volume: f64,
    pub guy_palette: Vec<Rgba<f32>>,
    pub background_palette: Vec<Rgba<f32>>,
    pub beard_color: Rgba<f32>,
}

#[derive(Deref)]
pub struct Texture(#[deref] ugli::Texture);

impl std::borrow::Borrow<ugli::Texture> for Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}
impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl geng::LoadAsset for Texture {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let texture = <ugli::Texture as geng::LoadAsset>::load(geng, path);
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Texture(texture))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

pub struct GuyAssets {
    pub hat: HashMap<String, Texture>,
    pub face: HashMap<String, Texture>,
    pub robe: HashMap<String, Texture>,
    pub beard: HashMap<String, Texture>,
    pub custom: HashMap<String, Texture>,
}

impl geng::LoadAsset for GuyAssets {
    fn load(geng: &Geng, path: &std::path::Path) -> geng::AssetFuture<Self> {
        let geng = geng.clone();
        let path = path.to_owned();
        async move {
            let json = <String as geng::LoadAsset>::load(&geng, &path.join("_list.json"))
                .await
                .context("Failed to load config")?;
            #[derive(Deserialize)]
            struct Config {
                hat: Vec<String>,
                face: Vec<String>,
                robe: Vec<String>,
                beard: Vec<String>,
                custom: Vec<String>,
            }
            let config: Config = serde_json::from_str(&json)?;
            let geng = &geng;
            let path = &path;
            let load_map = |class: String, list: Vec<String>| async move {
                Ok::<_, anyhow::Error>(
                    future::join_all(list.iter().map(move |name| {
                        <Texture as geng::LoadAsset>::load(
                            &geng,
                            &path.join(&class).join(format!("{}.png", name)),
                        )
                        .map(move |texture| (name, texture))
                    }))
                    .await
                    .into_iter()
                    .map(move |(name, texture)| {
                        texture.map(|texture| (name.clone(), texture)).unwrap()
                    })
                    .collect::<HashMap<String, Texture>>(),
                )
            };
            Ok(Self {
                hat: load_map("hat".to_owned(), config.hat)
                    .await
                    .context("Failed to load outfits")?,
                robe: load_map("robe".to_owned(), config.robe)
                    .await
                    .context("Failed to load outfits")?,
                face: load_map("face".to_owned(), config.face)
                    .await
                    .context("Failed to load faces")?,
                beard: load_map("beard".to_owned(), config.beard)
                    .await
                    .context("Failed to load outfits")?,
                custom: load_map("custom".to_owned(), config.custom)
                    .await
                    .context("Failed to load outfits")?,
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
}

#[derive(geng::Assets)]
pub struct Assets {
    pub fireball: Texture,
    pub background: ugli::Texture,
    #[asset(range = "1..=1", path = "background_entities/*.png")]
    pub background_entities: Vec<Texture>,
    pub config: Config,
    #[asset(path = "kuvimanPreBattle.wav")]
    pub lobby_music: geng::Sound,
    #[asset(path = "kuvimanBattle.wav")]
    pub battle_music: geng::Sound,
    #[asset(range = "1..=3", path = "player_joined*.mp3")]
    pub spawn_sfx: Vec<geng::Sound>,
    #[asset(path = "death.wav")]
    pub death_sfx: geng::Sound,
    #[asset(path = "victory.mp3")]
    pub win_sfx: geng::Sound,
    #[asset(path = "RaffleRoyaleTitle.wav")]
    pub title_sfx: geng::Sound,
    #[asset(path = "levelup.wav")]
    pub levelup_sfx: geng::Sound,
    pub levelup: Rc<Texture>,
    pub levelup_front: Rc<Texture>,
    pub skull: Rc<Texture>,
    pub guy: GuyAssets,
}

impl Assets {
    pub fn process(&mut self) {
        self.lobby_music.looped = true;
        self.battle_music.looped = true;
        self.background.set_filter(ugli::Filter::Nearest);
        self.background.set_wrap_mode(ugli::WrapMode::Repeat);
    }
}

type Id = i32;

#[derive(HasId)]
struct Guy {
    id: Id,
    name: String,
    health: usize,
    max_health: usize,
    velocity: Vec2<f32>,
    position: Vec2<f32>,
    spawn: f32,
    face: String,
    hat: String,
    robe: String,
    beard: String,
    custom: Option<String>,
    outfit_color: Rgba<f32>,
}

#[derive(Debug)]
struct Attack {
    attacker_id: Id,
    target_id: Id,
}

struct Circle {
    center: Vec2<f32>,
    radius: f32,
}

struct DelayedMessage {
    time: f32,
    message: String,
}

struct BackgroundEntity {
    texture_index: usize,
    position: Vec2<f32>,
    color: Rgba<f32>,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum RaffleMode {
    Regular,
    Ld,
}

pub struct State {
    opt: Opt,
    geng: Geng,
    assets: Rc<Assets>,
    guys: Collection<Guy>,
    camera: geng::Camera2d,
    framebuffer_size: Vec2<usize>,
    next_id: Id,
    process_battle: bool,
    winning_screen: bool,
    next_attack: Option<f32>,
    attacks: Vec<Attack>,
    queued_attack: Option<Attack>,
    circle: Circle,
    ttv_client: ttv::Client,
    feed: Option<String>,
    time: f32,
    delayed_messages: Vec<DelayedMessage>,
    lobby_music: geng::SoundEffect,
    battle_music: geng::SoundEffect,
    battle_fade: f32,
    victory_fade: f32,
    idle: bool,
    idle_fade: f32,
    db: Db,
    background_entities: Vec<BackgroundEntity>,
    raffle_mode: RaffleMode,
    effects: Vec<Effect>,
}

struct Effect {
    pos: Vec2<f32>,
    offset: f32,
    size: f32,
    time: f32,
    max_time: f32,
    back_texture: Option<Rc<Texture>>,
    front_texture: Option<Rc<Texture>>,
    guy_id: Option<Id>,
    color: Rgba<f32>,
}

impl Drop for State {
    fn drop(&mut self) {
        if !self.opt.no_chat_spam {
            self.ttv_client.say("Going to sleep 💤");
        }
    }
}

impl State {
    const GUY_RADIUS: f32 = 1.0;
    const MIN_DISTANCE: f32 = 5.0;
    const GUY_MAX_SPEED: f32 = 10.0;
    const GUY_ACCELERATION: f32 = 10.0;
    pub fn new(geng: &Geng, assets: &Rc<Assets>, ttv_client: ttv::Client, opt: Opt) -> Self {
        if !opt.no_chat_spam {
            ttv_client.say("Hai, im online 🤖");
        }
        let mut lobby_music = assets.lobby_music.effect();
        lobby_music.set_volume(0.0);
        lobby_music.play();
        let mut battle_music = assets.battle_music.effect();
        battle_music.set_volume(0.0);
        battle_music.play();

        Self {
            opt,
            idle: true,
            idle_fade: 0.0,
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
            circle: Circle {
                center: Vec2::ZERO,
                radius: 1.0,
            },
            ttv_client,
            winning_screen: false,
            feed: None,
            time: 0.0,
            delayed_messages: vec![],
            lobby_music,
            battle_music,
            battle_fade: 0.0,
            db: Db::new(&Secrets::read().unwrap().config.db.url),
            victory_fade: 0.0,
            background_entities: std::iter::from_fn(|| {
                let d = 50.0;
                Some(BackgroundEntity {
                    texture_index: global_rng().gen_range(0..assets.background_entities.len()),
                    position: vec2(global_rng().gen_range(-d..d), global_rng().gen_range(-d..d)),
                    color: *assets
                        .config
                        .background_palette
                        .choose(&mut global_rng())
                        .unwrap(),
                })
            })
            .take(500)
            .collect(),
            raffle_mode: RaffleMode::Regular,
            effects: vec![],
        }
    }

    fn find_circle(&self) -> Option<Circle> {
        let mut sum = Vec2::ZERO;
        let mut sum_spawns = 0.0;
        for guy in &self.guys {
            sum += guy.position * guy.spawn;
            sum_spawns += guy.spawn;
        }
        if sum_spawns == 0.0 {
            return None;
        }

        let center = sum / sum_spawns;

        let radius = self
            .guys
            .iter()
            .map(|guy| r32(((guy.position - center).len() + Self::GUY_RADIUS * 2.0) * guy.spawn))
            .max()
            .unwrap()
            .raw();
        Some(Circle { center, radius })
    }

    fn process_movement(&mut self, delta_time: f32) {
        let Circle { center, .. } = match self.find_circle() {
            Some(circle) => circle,
            None => return,
        };
        let ids = self.guys.ids().copied().collect::<Vec<_>>();

        // Guys do be accelerating
        for id in &ids {
            let mut guy = self.guys.remove(id).unwrap();
            let target_velocity =
                (center - guy.position).normalize_or_zero() * State::GUY_MAX_SPEED;
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
                    if len < State::MIN_DISTANCE {
                        let v = delta_pos.normalize_or_zero();
                        moves.push((guy.id, v * (State::MIN_DISTANCE - len) / 2.0));
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

    fn process_attacks(&mut self, delta_time: f32) {
        if !self.process_battle {
            self.feed = None;
            return;
        }
        if let Some(time) = &mut self.next_attack {
            *time -= delta_time * 3.0;
            if *time <= 0.0 {
                for attack in self.attacks.drain(..) {
                    self.guys.get_mut(&attack.target_id).unwrap().health -= 1;
                }
                for guy in &self.guys {
                    if guy.health == 0 {
                        self.feed = Some(format!("{} has been eliminated", guy.name));

                        self.effects.push(Effect {
                            pos: guy.position,
                            offset: 0.0,
                            size: 1.0,
                            time: 0.0,
                            max_time: 0.7,
                            back_texture: None,
                            front_texture: Some(self.assets.skull.clone()),
                            guy_id: Some(guy.id),
                            color: Rgba::BLACK,
                        });

                        let mut sound_effect = self.assets.death_sfx.effect();
                        sound_effect.set_volume(self.assets.config.volume * 0.5);
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

    fn spawn_guy(&mut self, name: String, random: bool) {
        let level = self.db.find_level(&name, !random);
        let health = self.assets.config.initial_health
            + level * self.assets.config.health_increase_per_level;
        let id = self.next_id;
        self.next_id += 1;
        self.guys.insert(Guy {
            id,
            name,
            position: std::iter::from_fn(|| {
                Some(
                    self.camera.center
                        + vec2(
                            global_rng().gen_range(
                                0.0..self.camera.fov / 2.0
                                    * (self.framebuffer_size.x as f32
                                        / self.framebuffer_size.y as f32)
                                        .max(1.0),
                            ),
                            0.0,
                        )
                        .rotate(global_rng().gen_range(0.0..2.0 * f32::PI)),
                )
            })
            .take(50)
            .filter(|&pos| {
                for guy in &self.guys {
                    if (guy.position - pos).len() < State::MIN_DISTANCE {
                        return false;
                    }
                }
                true
            })
            .min_by_key(|&pos| r32((pos - self.circle.center).len()))
            .unwrap_or(self.camera.center),
            velocity: Vec2::ZERO,
            health,
            max_health: health,
            spawn: 0.0,
            face: self
                .assets
                .guy
                .face
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            hat: self
                .assets
                .guy
                .hat
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            robe: self
                .assets
                .guy
                .robe
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            beard: self
                .assets
                .guy
                .beard
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            custom: None,
            outfit_color: *self
                .assets
                .config
                .guy_palette
                .choose(&mut global_rng())
                .unwrap(),
        });

        let mut sound_effect = self
            .assets
            .spawn_sfx
            .choose(&mut global_rng())
            .unwrap()
            .effect();
        sound_effect.set_volume(self.assets.config.volume);
        sound_effect.play();
    }

    fn start_raffle(&mut self, mode: RaffleMode) {
        if !self.idle {
            self.ttv_client.say("Raffle Royale is already going on");
            return;
        }
        if !self.opt.no_chat_spam {
            self.ttv_client
                .say("Raffle Royale is about to begin! Type !fight to join!");
        }
        self.idle = false;
        self.guys.clear();
        let mut sfx = self.assets.title_sfx.effect();
        sfx.set_volume(self.assets.config.volume * 3.0);
        sfx.play();
        self.raffle_mode = mode;
    }
}

impl geng::State for State {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(self.assets.config.background), None, None);

        {
            let vertex = |x, y| {
                let x = x * 2 - 1;
                let y = y * 2 - 1;
                let p = self.camera.center + vec2(x as f32, y as f32) * self.camera.fov * 2.0;
                draw_2d::TexturedVertex {
                    a_pos: p,
                    a_color: Rgba::WHITE,
                    a_vt: p,
                }
            };
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedPolygon::new(
                    vec![vertex(0, 0), vertex(0, 1), vertex(1, 1), vertex(1, 0)],
                    &self.assets.background,
                ),
            );
            for entity in &self.background_entities {
                let texture = &self.assets.background_entities[entity.texture_index];
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &draw_2d::TexturedQuad::unit_colored(texture, entity.color)
                        .scale(texture.size().map(|x| x as f32) / 128.0)
                        .translate(entity.position),
                );
            }
        }

        if self.idle {
            return;
        }

        // self.geng.draw_2d(
        //     framebuffer,
        //     &self.camera,
        //     &draw_2d::Ellipse::circle(
        //         self.circle.center,
        //         self.circle.radius,
        //         self.assets.config.circle,
        //     ),
        // );

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

        for effect in &self.effects {
            let t = effect.time / effect.max_time;
            if let Some(texture) = &effect.back_texture {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::unit_colored(
                        &**texture,
                        Rgba {
                            a: (1.0 - t) * effect.color.a,
                            ..effect.color
                        },
                    )
                    .scale_uniform(1.0 + t * effect.size)
                    .translate(effect.pos + vec2(0.0, -effect.offset)),
                );
            }
        }

        for guy in &self.guys {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::new(
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.guy.face[&guy.face],
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::colored(
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.guy.hat[&guy.hat],
                    guy.outfit_color,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::colored(
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.guy.robe[&guy.robe],
                    guy.outfit_color,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::colored(
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.guy.beard[&guy.beard],
                    self.assets.config.beard_color,
                ),
            );
            let hp_text_aabb =
                AABB::point(guy.position + vec2(-State::GUY_RADIUS, State::GUY_RADIUS) * 1.5)
                    .extend_uniform(State::GUY_RADIUS * 0.5);
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Ellipse::unit(Rgba::BLACK).fit_into(hp_text_aabb),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Text::unit(
                    &**self.geng.default_font(),
                    format!("{}/{}", guy.health, guy.max_health),
                    Hsva::new(
                        guy.health as f32 / guy.max_health as f32 / 3.0,
                        1.0,
                        1.0,
                        1.0,
                    )
                    .into(),
                )
                .fit_into(hp_text_aabb.extend_uniform(-0.2)),
            );

            let hp_bar_aabb = AABB::point(guy.position + vec2(0.0, State::GUY_RADIUS) * 1.5)
                .extend_symmetric(vec2(State::GUY_RADIUS, State::GUY_RADIUS * 0.1));
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(hp_bar_aabb.extend_uniform(0.1), Rgba::BLACK),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(hp_bar_aabb, Rgba::RED),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Quad::new(
                    AABB {
                        x_max: hp_bar_aabb.x_min
                            + hp_bar_aabb.width() * guy.health as f32 / guy.max_health as f32,
                        ..hp_bar_aabb
                    },
                    Rgba::GREEN,
                ),
            );

            let name_aabb = AABB::point(guy.position + vec2(0.0, State::GUY_RADIUS) * 2.0)
                .extend_symmetric(vec2(State::GUY_RADIUS * 1.0, State::GUY_RADIUS * 0.2));
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::Text::unit(&**self.geng.default_font(), &guy.name, Rgba::BLACK)
                    .fit_into(name_aabb),
            );
        }

        for effect in &self.effects {
            let t = effect.time / effect.max_time;
            if let Some(texture) = &effect.front_texture {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::unit_colored(
                        &**texture,
                        Rgba {
                            a: (1.0 - t) * effect.color.a,
                            ..effect.color
                        },
                    )
                    .scale_uniform(1.0 + t * effect.size)
                    .translate(effect.pos + vec2(0.0, -1.0)),
                );
            }
        }

        let ui_camera = geng::Camera2d {
            center: Vec2::ZERO,
            rotation: 0.0,
            fov: 15.0,
        };
        if !self.process_battle {
            self.winning_screen = false;
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "RAFFLE ROYALE",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .translate(vec2(0.0, 5.0)),
            );
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "WIP",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.5)
                .translate(vec2(12.0, 6.0)),
            );
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "type !fight to join",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.5)
                .translate(vec2(0.0, 2.5)),
            );
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "code - kuviman",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.2)
                .translate(vec2(0.0, -6.5)),
            );
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "musix&sfx - BrainoidGames",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.2)
                .translate(vec2(0.0, -7.0)),
            );
        } else if self.guys.len() == 1 {
            let winner = self.guys.iter().next().unwrap();
            if !self.winning_screen {
                if !self.opt.no_chat_spam {
                    match self.raffle_mode {
                        RaffleMode::Regular => {
                            self.delayed_messages.push(DelayedMessage {
                                time: self.time + 5.0,
                                message: format!("Winner is {} 🎉", winner.name),
                            });
                        }
                        RaffleMode::Ld => match self.db.find_game_link(&winner.name) {
                            Some(game_link) => {
                                self.db.set_game_played(&winner.name, true);
                                self.delayed_messages.push(DelayedMessage {
                                    time: self.time + 5.0,
                                    message: format!(
                                        "Winner is {} 🎉 Now we play {}",
                                        winner.name, game_link
                                    ),
                                });
                            }
                            None => {
                                self.delayed_messages.push(DelayedMessage {
                                    time: self.time + 5.0,
                                    message: format!(
                                        "Winner is {} 🎉 No game was submitted? :(",
                                        winner.name
                                    ),
                                });
                            }
                        },
                    }
                }
                self.winning_screen = true;
                let mut sound_effect = self.assets.win_sfx.effect();
                sound_effect.set_volume(self.assets.config.volume);
                sound_effect.play();
            }
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    if winner.name == "kuviman" {
                        "RIGGED"
                    } else {
                        "WINNER"
                    },
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .translate(vec2(0.0, 5.0)),
            );
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    "hooray",
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.5)
                .translate(vec2(0.0, 2.5)),
            );
        } else {
            if let Some(feed) = &self.feed {
                self.geng.default_font().draw(
                    framebuffer,
                    &ui_camera,
                    feed,
                    vec2(0.0, 6.0),
                    geng::TextAlign::CENTER,
                    1.0,
                    Rgba::BLACK,
                );
                // self.geng.draw_2d(
                //     framebuffer,
                //     &ui_camera,
                //     &draw_2d::Text::unit(&**self.geng.default_font(), feed, Rgba::BLACK)
                //         .scale_uniform(0.5)
                //         .translate(vec2(0.0, 6.0)),
                // );
            }
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
                            iter.find(|guy| (guy.position - position).len() < State::GUY_RADIUS)
                        {
                            guy.health += self.assets.config.health_increase_per_level;
                            guy.max_health += self.assets.config.health_increase_per_level;
                            let mut effect = self.assets.levelup_sfx.effect();
                            effect.set_volume(self.assets.config.volume);
                            effect.play();
                            self.effects.push(Effect {
                                pos: guy.position,
                                offset: 1.0,
                                size: 1.0,
                                time: 0.0,
                                max_time: 1.35,
                                back_texture: Some(self.assets.levelup.clone()),
                                front_texture: Some(self.assets.levelup_front.clone()),
                                guy_id: Some(guy.id),
                                color: Rgba::YELLOW,
                            });
                        }
                    }
                    geng::MouseButton::Right => {
                        self.guys
                            .retain(|guy| (guy.position - position).len() > State::GUY_RADIUS);
                    }
                    _ => {}
                }
            }
            geng::Event::KeyDown { key } => match key {
                geng::Key::S => {
                    self.spawn_guy(
                        global_rng()
                            .sample_iter(rand::distributions::Alphanumeric)
                            .map(|c| c as char)
                            .take(global_rng().gen_range(5..=15))
                            .collect(),
                        true,
                    );
                }
                geng::Key::Space => {
                    if self.idle {
                        self.start_raffle(RaffleMode::Regular);
                    } else if !self.process_battle {
                        self.process_battle = true;
                    } else {
                        self.process_battle = false;
                        self.idle = true;
                    }
                }
                geng::Key::F11 => {
                    self.geng.window().toggle_fullscreen();
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;

        for effect in &mut self.effects {
            effect.time += delta_time;
            if let Some(id) = effect.guy_id {
                if let Some(guy) = self.guys.get(&id) {
                    effect.pos = guy.position;
                }
            }
        }
        self.effects.retain(|effect| effect.time < effect.max_time);

        let was_idle = self.idle_fade != 1.0;
        // TODO: window.is_minimized?
        let volume = if self.idle {
            self.idle_fade = 0.0;
            0.0
        } else {
            self.idle_fade = (self.idle_fade + delta_time / 2.5).min(1.0);
            self.assets.config.volume * if self.idle_fade == 1.0 { 1.0 } else { 0.0 }
        };
        let start_music = was_idle && self.idle_fade == 1.0;
        if start_music {
            self.lobby_music.pause();
            self.battle_music.pause();
            self.lobby_music = self.assets.lobby_music.effect();
            self.battle_music = self.assets.battle_music.effect();
        }
        if self.process_battle {
            self.battle_fade += delta_time;
            if self.guys.len() == 1 {
                self.victory_fade = (self.victory_fade + delta_time).min(1.0);
            }
        } else {
            self.victory_fade = 0.0;
            self.battle_fade -= delta_time;
        }
        self.battle_fade = self.battle_fade.clamp(0.0, 1.0);
        self.battle_music
            .set_volume(self.battle_fade as f64 * volume * (1.0 - self.victory_fade as f64));
        self.lobby_music
            .set_volume((1.0 - self.battle_fade as f64) * volume);

        if start_music {
            self.lobby_music.play();
            self.battle_music.play();
        }

        self.time += delta_time;
        for message in &self.delayed_messages {
            if message.time <= self.time {
                self.ttv_client.say(&message.message);
            }
        }
        self.delayed_messages
            .retain(|message| message.time > self.time);

        self.process_movement(delta_time);
        self.process_attacks(delta_time);

        for guy in &mut self.guys {
            guy.spawn = (guy.spawn + delta_time).min(1.0);
        }

        if let Some(target_circle) = self.find_circle() {
            self.circle.center += (target_circle.center - self.circle.center) * delta_time;
            self.circle.radius += (target_circle.radius - self.circle.radius) * delta_time;
        }
        let mut target_fov = self.circle.radius * 2.5;
        let mut target_center = self.circle.center;
        if !self.process_battle || self.guys.len() == 1 {
            target_center.y += target_fov * 0.3;
            target_fov *= 1.5;
        }
        self.camera.center += (target_center - self.camera.center) * delta_time;
        self.camera.fov += (target_fov - self.camera.fov) * delta_time;

        while let Some(message) = self.ttv_client.next_message() {
            match message {
                ttv::Message::Irc(ttv::IrcMessage::Privmsg(message)) => {
                    let mut name = message.sender.name.as_str();
                    let mut message_text = message.message_text.as_str();
                    if name == "kuviman" {
                        if let Some(text) = message_text.strip_prefix("!as") {
                            if let Some((as_name, text)) = text.trim().split_once(' ') {
                                name = as_name.trim();
                                message_text = text.trim();
                            }
                        }
                    }
                    if let Some(url) = message_text.strip_prefix("!submit") {
                        let url = url.trim();
                        if url.is_empty() {
                            self.ttv_client
                                .reply("Submit using !submit <url>", &message);
                        } else {
                            if self.db.game_played(name) {
                                self.ttv_client
                                    .reply("We have already played your game", &message);
                            } else {
                                if self.db.find_game_link(name).is_some() {
                                    self.ttv_client
                                        .reply("You have already submitted a game tho", &message);
                                } else {
                                    self.db.set_game_link(name, Some(url));
                                    self.ttv_client.reply("Submission successful", &message);
                                }
                            }
                        }
                    }
                    match message_text.trim() {
                        "!fight" | "!join" => {
                            if self.idle {
                                self.ttv_client
                                    .reply("There is no raffle going on right now", &message);
                            } else if !self.process_battle {
                                if self.guys.iter().any(|guy| guy.name == name) {
                                    self.ttv_client.reply("No cheating allowed 🚫", &message);
                                } else {
                                    if self.raffle_mode == RaffleMode::Ld
                                        && self.db.find_game_link(name).is_none()
                                    {
                                        self.ttv_client
                                            .reply("You should !submit first!", &message);
                                    } else {
                                        self.spawn_guy(name.to_owned(), false);
                                    }
                                }
                            } else {
                                self.ttv_client.reply(
                                    "You can't join into an ongoing fight, sorry Kappa",
                                    &message,
                                );
                            }
                        }
                        "!lvl" | "!level" => {
                            let level = self.db.find_level(&name, true);
                            let hp = self.assets.config.initial_health
                                + level * self.assets.config.health_increase_per_level;
                            self.ttv_client.reply(
                                &format!("You are level {} ({} hp) ⭐", level, hp),
                                &message,
                            );
                        }
                        "!raffle royale" if name == "kuviman" => {
                            self.start_raffle(RaffleMode::Regular);
                        }
                        "!raffle royale ld" if name == "kuviman" => {
                            self.start_raffle(RaffleMode::Ld);
                        }
                        _ => {}
                    }
                }
                ttv::Message::RewardRedemption { name, reward } => {
                    if reward == "Raffle Royale Level Up" {
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.health += self.assets.config.health_increase_per_level;
                            guy.max_health += self.assets.config.health_increase_per_level;
                            let mut effect = self.assets.levelup_sfx.effect();
                            effect.set_volume(self.assets.config.volume);
                            effect.play();

                            self.effects.push(Effect {
                                pos: guy.position,
                                offset: 1.0,
                                size: 1.0,
                                time: 0.0,
                                max_time: 1.35,
                                back_texture: Some(self.assets.levelup.clone()),
                                front_texture: Some(self.assets.levelup_front.clone()),
                                guy_id: Some(guy.id),
                                color: Rgba::YELLOW,
                            });
                        }
                        let level = self.db.find_level(&name, false) + 1;
                        self.db.set_level(&name, level);
                        let hp = self.assets.config.initial_health
                            + level * self.assets.config.health_increase_per_level;
                        self.ttv_client
                            .say(&format!("{} is now level {} ({} hp) ⭐", name, level, hp));
                    }
                }
                _ => {}
            }
        }
    }
}
