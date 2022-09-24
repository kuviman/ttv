use super::*;

#[derive(Deserialize, geng::Assets)]
#[asset(json)]
pub struct Config {
    pub background: Rgba<f32>,
    pub circle: Rgba<f32>,
    pub initial_health: usize,
    pub health_increase_per_level: usize,
    pub volume: f64,
}

#[derive(geng::Assets)]
pub struct Assets {
    pub hat: ugli::Texture,
    pub face: ugli::Texture,
    pub fireball: ugli::Texture,
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
}

impl Assets {
    pub fn process(&mut self) {
        self.lobby_music.looped = true;
        self.battle_music.looped = true;
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

pub struct State {
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
    db: Db,
}

impl Drop for State {
    fn drop(&mut self) {
        self.ttv_client.say("Going to sleep 💤");
    }
}

impl State {
    const GUY_RADIUS: f32 = 1.0;
    const MIN_DISTANCE: f32 = 5.0;
    const GUY_MAX_SPEED: f32 = 10.0;
    const GUY_ACCELERATION: f32 = 10.0;
    pub fn new(geng: &Geng, assets: &Rc<Assets>, ttv_client: ttv::Client) -> Self {
        ttv_client.say("Hai, im online 🤖");
        let mut lobby_music = assets.lobby_music.effect();
        lobby_music.set_volume(0.0);
        lobby_music.play();
        let mut battle_music = assets.battle_music.effect();
        battle_music.set_volume(0.0);
        battle_music.play();

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

                        let mut sound_effect = self.assets.death_sfx.effect();
                        sound_effect.set_volume(self.assets.config.volume);
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

    fn spawn_guy(&mut self, name: String) {
        let level = self.db.find_level(&name);
        let health = self.assets.config.initial_health
            + level * self.assets.config.health_increase_per_level;
        let id = self.next_id;
        self.next_id += 1;
        self.guys.insert(Guy {
            id,
            name,
            position: self.camera.center
                + vec2(
                    self.camera.fov / 2.0
                        * (self.framebuffer_size.x as f32 / self.framebuffer_size.y as f32)
                            .max(1.0),
                    0.0,
                )
                .rotate(global_rng().gen_range(0.0..2.0 * f32::PI)),
            velocity: Vec2::ZERO,
            health,
            max_health: health,
            spawn: 0.0,
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
}

impl geng::State for State {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(self.assets.config.background), None, None);

        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::Ellipse::circle(
                self.circle.center,
                self.circle.radius,
                self.assets.config.circle,
            ),
        );

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
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.face,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &geng::draw_2d::TexturedQuad::new(
                    AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                    &self.assets.hat,
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
                self.delayed_messages.push(DelayedMessage {
                    time: self.time + 5.0,
                    message: format!("Winner is {} 🎉", winner.name),
                });
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
                    );
                }
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

        // TODO: window.is_minimized?
        let volume = if self.geng.window().real_size() == vec2(0, 0) {
            0.0
        } else {
            self.assets.config.volume
        };
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
                    let name = message.sender.name.as_str();
                    match message.message_text.trim() {
                        "!fight" | "!join" => {
                            if !self.process_battle {
                                if self.guys.iter().any(|guy| guy.name == name) {
                                    self.ttv_client.reply("No cheating allowed 🚫", &message);
                                } else {
                                    self.spawn_guy(name.to_owned());
                                }
                            } else {
                                self.ttv_client.reply(
                                    "You can't join into an ongoing fight, sorry Kappa",
                                    &message,
                                );
                            }
                        }
                        "!lvl" | "!level" => {
                            let level = self.db.find_level(&name);
                            let hp = self.assets.config.initial_health
                                + level * self.assets.config.health_increase_per_level;
                            self.ttv_client.reply(
                                &format!("You are level {} ({} hp) ⭐", level, hp),
                                &message,
                            );
                        }
                        _ => {}
                    }
                }
                ttv::Message::RewardRedemption { name, reward } => {
                    if reward == "Raffle Royale Level Up" {
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.health += self.assets.config.health_increase_per_level;
                            guy.max_health += self.assets.config.health_increase_per_level;
                        }
                        let level = self.db.find_level(&name) + 1;
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
