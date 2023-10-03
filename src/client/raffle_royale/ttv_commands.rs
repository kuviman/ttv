use super::*;

impl State {
    pub async fn handle_message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::ChatMessage {
                id: message_id,
                name,
                message,
            } => {
                let mut name = name.as_str();
                let mut message_text = message.as_str();
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
                        self.connection
                            .reply("Submit using !submit <url> 🔗", &message_id);
                    } else if self.db.game_played(name).await {
                        self.connection
                            .reply("We have already played your game 😕", &message_id);
                    } else if self.db.find_game_link(name).await.is_some() {
                        self.connection
                            .reply("You have already submitted a game tho 😕", &message_id);
                    } else {
                        self.db.set_game_link(name, Some(url));

                        let mut text = "Submission successful 👌".to_owned();
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            if guy.should_never_win {
                                guy.should_never_win = false;
                                text += " Your curse has been reversed";
                            }
                        }
                        self.connection.reply(&text, &message_id);
                    }
                }
                if let Some(hat) = message_text.strip_prefix("!hat") {
                    let hat = hat.trim();
                    if self.assets.guy.hat.contains_key(hat) {
                        let mut skin = self.find_skin(name, false).await;
                        skin.hat = hat.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.connection.reply(
                            &format!(
                                "⚙️ Hat options: {}",
                                self.assets
                                    .guy
                                    .hat
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message_id,
                        );
                    }
                }
                if let Some(parts) = message_text.strip_prefix("!setcustomskin") {
                    if name == "kuviman" {
                        let mut parts = parts.split_whitespace();
                        if let Some(name) = parts.next() {
                            if let Some(custom) = parts.next() {
                                if self.assets.guy.custom.contains_key(custom) {
                                    let mut skin = self.find_skin(name, false).await;
                                    skin.custom = Some(custom.to_owned());
                                    self.db.set_skin(name, &skin);
                                    if let Some(guy) =
                                        self.guys.iter_mut().find(|guy| guy.name == name)
                                    {
                                        guy.skin = skin;
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(face) = message_text.strip_prefix("!face") {
                    let face = face.trim();
                    if self.assets.guy.face.contains_key(face) {
                        let mut skin = self.find_skin(name, false).await;
                        skin.face = face.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.connection.reply(
                            &format!(
                                "⚙️ Face options: {}",
                                self.assets
                                    .guy
                                    .face
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message_id,
                        );
                    }
                }
                if let Some(robe) = message_text.strip_prefix("!robe") {
                    let robe = robe.trim();
                    if self.assets.guy.robe.contains_key(robe) {
                        let mut skin = self.find_skin(name, false).await;
                        skin.robe = robe.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.connection.reply(
                            &format!(
                                "⚙️ Robe options: {}",
                                self.assets
                                    .guy
                                    .robe
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message_id,
                        );
                    }
                }
                if let Some(beard) = message_text.strip_prefix("!beard") {
                    let beard = beard.trim();
                    if self.assets.guy.beard.contains_key(beard) {
                        let mut skin = self.find_skin(name, false).await;
                        skin.beard = beard.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.connection.reply(
                            &format!(
                                "⚙️ Beard options: {}",
                                self.assets
                                    .guy
                                    .beard
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message_id,
                        );
                    }
                }
                if message_text.trim() == format!("!{}", self.raffle_keyword) {
                    if self.idle {
                        self.connection
                            .reply("You are either too late or too early 😊", &message_id);
                    } else if !self.process_battle {
                        if self.guys.iter().any(|guy| guy.name == name) {
                            self.connection.reply("No cheating allowed 🚫", &message_id);
                        } else {
                            self.spawn_guy(name.to_owned(), false).await;
                            if self.raffle_mode == RaffleMode::Ld
                                && self.db.find_game_link(name).await.is_none()
                            {
                                self.connection.reply("You didn't !submit a game so you are cursed. Submit to reverse it ⏳", &message_id);
                            }
                        }
                    } else {
                        self.connection.reply(
                            "You can't join into an ongoing fight, sorry Kappa",
                            &message_id,
                        );
                    }
                }
                if let Some(keyword) = message_text.strip_prefix("!raffle") {
                    if name == "kuviman" {
                        let keyword = keyword.trim();
                        if keyword.is_empty() {
                            self.start_raffle(RaffleMode::Ld);
                        } else if keyword == "start" {
                            if !self.idle {
                                self.levelup_all().await;
                                self.process_battle = true;
                            }
                        } else if keyword == "close" {
                            self.process_battle = false;
                            self.idle = true;
                        } else if let Some(keyword) = keyword.strip_prefix("royale") {
                            let keyword = keyword.trim();
                            if !keyword.is_empty() {
                                self.raffle_keyword = keyword.to_owned();
                            }
                            self.start_raffle(RaffleMode::Regular);
                        } else {
                            self.raffle_keyword = keyword.to_owned();
                            self.start_raffle(RaffleMode::Ld);
                        }
                    }
                }
                if name == "kuviman" {
                    if let Some(name) = message_text.strip_prefix("!curse") {
                        let name = name.trim();
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.should_never_win = true;
                        }
                    }
                    if let Some(name) = message_text.strip_prefix("!bless") {
                        let name = name.trim();
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.health += self.assets.constants.bless_hp;
                            guy.max_health += self.assets.constants.bless_hp;

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
                    if let Some(names) = message_text.strip_prefix("!spawn") {
                        for name in names.split_whitespace() {
                            self.spawn_guy(name.to_owned(), true).await;
                        }
                    }
                }
                match message_text.trim() {
                    "!lvl" | "!level" => {
                        let level = self.db.find_level(&name).await;
                        let hp = self.assets.constants.initial_health
                            + (level.max(1) - 1) * self.assets.constants.extra_health_per_level;
                        self.connection.reply(
                            &format!("You are level {} ({} hp) ⭐", level, hp),
                            &message_id,
                        );
                    }
                    "!skin" => {
                        let skin = self.find_skin(name, true).await;
                        self.connection.reply(&skin.to_string(), &message_id);
                    }
                    "!skin random" => {
                        let skin = Skin::random(&self.assets);
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    }
                    _ => {}
                }
            }
            ServerMessage::RewardRedemption { name, reward } => {
                if reward == "Raffle Royale Level Up" {
                    if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                        let extra_hp = self.assets.constants.extra_health_per_level
                            * self.assets.constants.channel_point_levels;
                        guy.health += extra_hp;
                        guy.max_health += extra_hp;
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
                    let level = self.db.find_level(&name).await + 1;
                    self.db.set_level(&name, level);
                    let hp = self.assets.constants.initial_health
                        + (level.max(1) - 1) * self.assets.constants.extra_health_per_level;
                    self.connection
                        .say(&format!("{} is now level {} ({} hp) ⭐", name, level, hp));
                }
            }
            _ => {}
        }
    }
}
