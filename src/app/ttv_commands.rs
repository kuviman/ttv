use super::*;

impl State {
    pub fn handle_ttv(&mut self, message: ttv::Message) {
        match message {
            ttv::Message::Irc(ttv::IrcMessage::Privmsg(message)) => {
                let mut name = message.sender.name.as_str();
                let mut message_text = message.message_text.as_str();
                if name == self.config.channel_login {
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
                if let Some(hat) = message_text.strip_prefix("!hat") {
                    let hat = hat.trim();
                    if self.assets.guy.hat.contains_key(hat) {
                        let mut skin = self.find_skin(name, false);
                        skin.hat = hat.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.ttv_client.reply(
                            &format!(
                                "Hat options: {}",
                                self.assets
                                    .guy
                                    .hat
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message,
                        );
                    }
                }
                if let Some(parts) = message_text.strip_prefix("!setcustomskin") {
                    if name == self.config.channel_login {
                        let mut parts = parts.split_whitespace();
                        if let Some(name) = parts.next() {
                            if let Some(custom) = parts.next() {
                                if self.assets.guy.custom.contains_key(custom) {
                                    let mut skin = self.find_skin(name, false);
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
                        let mut skin = self.find_skin(name, false);
                        skin.face = face.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.ttv_client.reply(
                            &format!(
                                "Face options: {}",
                                self.assets
                                    .guy
                                    .face
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message,
                        );
                    }
                }
                if let Some(robe) = message_text.strip_prefix("!robe") {
                    let robe = robe.trim();
                    if self.assets.guy.robe.contains_key(robe) {
                        let mut skin = self.find_skin(name, false);
                        skin.robe = robe.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.ttv_client.reply(
                            &format!(
                                "Robe options: {}",
                                self.assets
                                    .guy
                                    .robe
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message,
                        );
                    }
                }
                if let Some(beard) = message_text.strip_prefix("!beard") {
                    let beard = beard.trim();
                    if self.assets.guy.beard.contains_key(beard) {
                        let mut skin = self.find_skin(name, false);
                        skin.beard = beard.to_owned();
                        self.db.set_skin(name, &skin);
                        if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                            guy.skin = skin;
                        }
                    } else {
                        self.ttv_client.reply(
                            &format!(
                                "Beard options: {}",
                                self.assets
                                    .guy
                                    .beard
                                    .keys()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            &message,
                        );
                    }
                }
                if message_text.trim() == format!("!{}", self.raffle_keyword) {
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
                                self.ttv_client.reply("You should !submit first!", &message);
                            } else if self.raffle_mode == RaffleMode::Ld
                                && self.db.game_played(name)
                            {
                                // self.ttv_client.reply("You shall not win", &message);
                                self.spawn_guy(name.to_owned(), false);
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
                if let Some(keyword) = message_text.strip_prefix("!raffle") {
                    if name == self.config.channel_login {
                        let keyword = keyword.trim();
                        if keyword.is_empty() {
                            self.start_raffle(RaffleMode::Ld);
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
                match message_text.trim() {
                    "!lvl" | "!level" => {
                        let level = self.db.find_level(&name);
                        let hp = level * self.assets.constants.health_per_level;
                        self.ttv_client
                            .reply(&format!("You are level {} ({} hp) ⭐", level, hp), &message);
                    }
                    "!skin" => {
                        let skin = self.find_skin(name, true);
                        self.ttv_client.reply(&skin.to_string(), &message);
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
            ttv::Message::RewardRedemption { name, reward } => {
                if reward == "Raffle Royale Level Up" {
                    if let Some(guy) = self.guys.iter_mut().find(|guy| guy.name == name) {
                        guy.health += self.assets.constants.health_per_level;
                        guy.max_health += self.assets.constants.health_per_level;
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
                    let level = self.db.find_level(&name) + 1;
                    self.db.set_level(&name, level);
                    let hp = level * self.assets.constants.health_per_level;
                    self.ttv_client
                        .say(&format!("{} is now level {} ({} hp) ⭐", name, level, hp));
                }
            }
            _ => {}
        }
    }
}
