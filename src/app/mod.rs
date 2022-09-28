use super::*;

mod assets;
mod attacks;
mod draw;
mod movement;
mod spawn;
mod ttv_commands;

pub use assets::*;

type Id = i32;

#[derive(HasId)]
struct Guy {
    id: Id,
    name: String,
    health: usize,
    max_health: usize,
    velocity: Vec2<f32>,
    position: Vec2<f32>,
    skin: Skin,
    spawn: f32,
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
    config: Config,
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
    scale_up: f32,
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
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        config: Config,
        ttv_client: ttv::Client,
        opt: Opt,
    ) -> Self {
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
            config,
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
            db: Db::new(&Secrets::init().unwrap().config.db.url),
            victory_fade: 0.0,
            background_entities: std::iter::from_fn(|| {
                let d = 50.0;
                Some(BackgroundEntity {
                    texture_index: global_rng().gen_range(0..assets.background_entities.len()),
                    position: vec2(global_rng().gen_range(-d..d), global_rng().gen_range(-d..d)),
                    color: *assets
                        .constants
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
        self.attacks.clear();
        self.next_attack = None;
        self.queued_attack = None;
        let mut sfx = self.assets.title_sfx.effect();
        sfx.set_volume(self.assets.constants.volume * 3.0);
        sfx.play();
        self.raffle_mode = mode;
    }

    fn find_skin(&self, name: &str, insert_if_absent: bool) -> Skin {
        if let Some(skin) = self.db.find_skin(name) {
            return skin;
        }
        let mut skin = Skin::random(&self.assets);
        if let Some(custom) = self.assets.guy.custom_map.get(name) {
            skin.custom = Some(custom.to_owned());
        }
        if insert_if_absent {
            self.db.set_skin(name, &skin);
        }
        skin
    }
    fn update_impl(&mut self, delta_time: f32) {
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
            self.assets.constants.volume * if self.idle_fade == 1.0 { 1.0 } else { 0.0 }
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
            self.handle_ttv(message);
        }
    }
}

impl geng::State for State {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.draw_impl(framebuffer);
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
                            guy.health += self.assets.constants.health_increase_per_level;
                            guy.max_health += self.assets.constants.health_increase_per_level;
                            let mut effect = self.assets.levelup_sfx.effect();
                            effect.set_volume(self.assets.constants.volume);
                            effect.play();
                            self.effects.push(Effect {
                                pos: guy.position,
                                scale_up: 0.2,
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
                        for guy in &self.guys {
                            if (guy.position - position).len() < State::GUY_RADIUS {
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
                                sound_effect.set_volume(self.assets.constants.volume * 0.5);
                                sound_effect.play();
                            }
                        }
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
        self.update_impl(delta_time as f32);
    }
}
