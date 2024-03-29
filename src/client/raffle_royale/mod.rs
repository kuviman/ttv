use super::*;

mod assets;
mod attacks;
mod db;
mod draw;
mod movement;
mod skin;
mod spawn;
mod ttv_commands;

use db::*;
use skin::Skin;

pub use assets::Assets;
pub use assets::*;

type Id = i32;

#[derive(HasId)]
struct Guy {
    id: Id,
    name: String,
    health: usize,
    max_health: usize,
    velocity: vec2<f32>,
    position: vec2<f32>,
    skin: Skin,
    spawn: f32,
    should_never_win: bool,
}

#[derive(Debug)]
struct Attack {
    attacker_id: Id,
    target_id: Id,
    hit: bool,
}

struct Circle {
    center: vec2<f32>,
    radius: f32,
}

struct DelayedMessage {
    time: f32,
    message: String,
}

struct BackgroundEntity {
    texture_index: usize,
    position: vec2<f32>,
    color: Rgba<f32>,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
enum RaffleMode {
    Regular,
    Ld,
}

pub struct State {
    connection: Connection,
    // opt: Opt,
    geng: Geng,
    assets: Rc<Assets>,
    // config: Config,
    guys: Collection<Guy>,
    camera: geng::Camera2d,
    framebuffer_size: vec2<usize>,
    next_id: Id,
    process_battle: bool,
    winning_screen: bool,
    next_attack: Option<f32>,
    attacks: Vec<Attack>,
    queued_attack: Option<Attack>,
    circle: Circle,
    // ttv_client: ttv::Client,
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
    raffle_keyword: String,
    volume: f64,
    noise: noise::OpenSimplex,
}

struct Effect {
    pos: vec2<f32>,
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
        // if !self.opt.no_chat_spam {
        self.connection.say("Going to sleep 💤");
        // }
    }
}

impl State {
    const GUY_RADIUS: f32 = 1.0;
    const MIN_DISTANCE: f32 = 2.0;
    const PREFERRED_DISTANCE: f32 = 5.0;
    const GUY_MAX_SPEED: f32 = 10.0;
    const GUY_ACCELERATION: f32 = 10.0;
    pub fn new(
        geng: &Geng,
        assets: &Rc<Assets>,
        connection: Connection,
        // config: Config,
        // ttv_client: ttv::Client,
        // opt: Opt,
    ) -> Self {
        // if !opt.no_chat_spam {
        connection.say("Hai, im online 🤖");
        // }
        let mut lobby_music = assets.lobby_music.effect();
        lobby_music.set_volume(0.0);
        lobby_music.play();
        let mut battle_music = assets.battle_music.effect();
        battle_music.set_volume(0.0);
        battle_music.play();

        Self {
            db: Db::new(connection.clone()),
            connection,
            volume: assets.constants.volume,
            // config,
            // opt,
            idle: true,
            idle_fade: 0.0,
            next_id: 0,
            geng: geng.clone(),
            assets: assets.clone(),
            guys: default(),
            camera: geng::Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 50.0,
            },
            framebuffer_size: vec2(1, 1),
            process_battle: false,
            next_attack: None,
            attacks: vec![],
            queued_attack: None,
            circle: Circle {
                center: vec2::ZERO,
                radius: 1.0,
            },
            // ttv_client,
            winning_screen: false,
            feed: None,
            time: 0.0,
            delayed_messages: vec![],
            lobby_music,
            battle_music,
            battle_fade: 0.0,
            // db: Db::new(&Secrets::init().unwrap().config.db.url),
            victory_fade: 0.0,
            background_entities: std::iter::from_fn(|| {
                let d = 50.0;
                Some(BackgroundEntity {
                    texture_index: thread_rng().gen_range(0..assets.background_entities.len()),
                    position: vec2(thread_rng().gen_range(-d..d), thread_rng().gen_range(-d..d)),
                    color: *assets
                        .constants
                        .background_palette
                        .choose(&mut thread_rng())
                        .unwrap(),
                })
            })
            .take(500)
            .collect(),
            raffle_mode: RaffleMode::Ld,
            effects: vec![],
            raffle_keyword: "fight".to_owned(),
            noise: noise::OpenSimplex::new(thread_rng().gen()),
        }
    }

    fn find_circle(&self) -> Option<Circle> {
        let mut sum = vec2::ZERO;
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

    async fn levelup_all(&self) {
        for guy in &self.guys {
            let current_level = self.db.find_level(&guy.name).await;
            if !guy.should_never_win {
                self.db.set_level(&guy.name, current_level + 1);
            }
        }
    }

    fn start_raffle(&mut self, mode: RaffleMode) {
        if !self.idle {
            self.connection.say("Raffle Royale is already going on 😕");
            return;
        }
        // if !self.opt.no_chat_spam {
        self.connection.say(&format!(
            "🧙‍♀️ Raffle Royale is about to begin! Type !{} to join! 🧙‍♂️",
            self.raffle_keyword
        ));
        // }
        self.idle = false;
        self.guys.clear();
        self.attacks.clear();
        self.next_attack = None;
        self.queued_attack = None;
        let mut sfx = self.assets.title_sfx.effect();
        sfx.set_volume((self.volume * 3.0).clamp(0.0, 1.0)); // TODO ?????
        sfx.play();
        self.raffle_mode = mode;
    }

    async fn find_skin(&self, name: &str, insert_if_absent: bool) -> Skin {
        if let Some(skin) = self.db.find_skin(name).await {
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
    async fn update_impl(&mut self, delta_time: f32) {
        if self.geng.window().is_key_pressed(geng::Key::PageUp) {
            self.volume = (self.volume + delta_time as f64).min(1.0);
        }
        if self.geng.window().is_key_pressed(geng::Key::PageDown) {
            self.volume = (self.volume - delta_time as f64).max(0.0);
        }

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
            self.volume * if self.idle_fade == 1.0 { 1.0 } else { 0.0 }
        };
        let start_music = was_idle && self.idle_fade == 1.0;
        if start_music {
            self.lobby_music.stop();
            self.battle_music.stop();
            self.lobby_music = self.assets.lobby_music.effect();
            self.battle_music = self.assets.battle_music.effect();
        }
        if self.process_battle {
            self.battle_fade += delta_time;
            if self.guys.len() == 1 {
                self.victory_fade = (self.victory_fade + delta_time).min(1.0);

                if !self.winning_screen {
                    let winner = self.guys.iter().next().unwrap();
                    if true {
                        // TODO !self.opt.no_chat_spam {
                        match self.raffle_mode {
                            RaffleMode::Regular => {
                                self.delayed_messages.push(DelayedMessage {
                                    time: self.time + 5.0,
                                    message: format!("Winner is {} 🎉", winner.name),
                                });
                            }
                            RaffleMode::Ld => match self.db.find_game_link(&winner.name).await {
                                Some(game_link) => {
                                    if self.db.game_played(&winner.name).await {
                                        self.delayed_messages.push(DelayedMessage {
                                        time: self.time + 5.0,
                                        message: format!(
                                            "Winner is {} 🎉 Your game ({}) was already played, please stop cheating?? 👀", 
                                            winner.name, game_link
                                        ),
                                    });
                                    } else {
                                        self.db.set_game_played(&winner.name, true);
                                        self.delayed_messages.push(DelayedMessage {
                                            time: self.time + 5.0,
                                            message: format!(
                                                "Winner is {} 🎉 Now we play {} 👏",
                                                winner.name, game_link
                                            ),
                                        });
                                    }
                                }
                                None => {
                                    self.delayed_messages.push(DelayedMessage {
                                        time: self.time + 5.0,
                                        message: format!(
                                            "Winner is {} 🎉 No game was submitted? 😔",
                                            winner.name
                                        ),
                                    });
                                }
                            },
                        }
                    }
                    self.winning_screen = true;
                    let mut sound_effect = self.assets.win_sfx.effect();
                    sound_effect.set_volume(self.volume);
                    sound_effect.play();
                }
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
                self.connection.say(&message.message);
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

        // while let Some(message) = self.ttv_client.next_message() {
        //     self.handle_ttv(message);
        // }
    }
}

#[async_trait(?Send)]
impl Feature for State {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.draw_impl(framebuffer);
    }
    async fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::MousePress { button } => {
                let position = self.geng.window().cursor_position().unwrap();
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
                            guy.health += self.assets.constants.health_per_click;
                            guy.max_health += self.assets.constants.health_per_click;
                            let mut effect = self.assets.levelup_sfx.effect();
                            effect.set_volume(self.volume);
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
                                sound_effect.set_volume(self.volume * 0.5);
                                sound_effect.play();
                            }
                        }
                        self.guys
                            .retain(|guy| (guy.position - position).len() > State::GUY_RADIUS);
                    }
                    _ => {}
                }
            }
            geng::Event::KeyPress { key } => match key {
                geng::Key::S => {
                    self.spawn_guy(
                        thread_rng()
                            .sample_iter(rand::distributions::Alphanumeric)
                            .map(|c| c as char)
                            .take(thread_rng().gen_range(5..=15))
                            .collect(),
                        true,
                    )
                    .await;
                }
                geng::Key::Space => {
                    if self.idle {
                        self.start_raffle(self.raffle_mode);
                    } else if !self.process_battle {
                        self.levelup_all().await;
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
    async fn update(&mut self, delta_time: f32) {
        self.update_impl(delta_time).await;
    }

    async fn load(geng: Geng, assets_path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        let mut assets: Assets =
            geng::asset::Load::load(geng.asset_manager(), &assets_path, &default())
                .await
                .unwrap();
        assets.process();
        Self::new(&geng, &Rc::new(assets), connection)
    }

    async fn handle(&mut self, message: &ServerMessage) {
        self.handle_message(message.clone()).await;
    }
}
