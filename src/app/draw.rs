use super::*;

impl State {
    pub fn draw_impl(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size();
        ugli::clear(
            framebuffer,
            Some(self.assets.constants.background),
            None,
            None,
        );

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
            let attacker = match self.guys.get(&attack.attacker_id) {
                Some(x) => x,
                None => continue,
            };
            let target = match self.guys.get(&attack.target_id) {
                Some(x) => x,
                None => continue,
            };
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
                    .scale_uniform(1.0 + t * effect.size * effect.scale_up)
                    .translate(effect.pos + vec2(0.0, -effect.offset)),
                );
            }
        }

        for guy in &self.guys {
            if let Some(custom) = &guy.skin.custom {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::new(
                        AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                        &self.assets.guy.custom[custom],
                    ),
                );
            } else {
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::new(
                        AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                        &self.assets.guy.face[&guy.skin.face],
                    ),
                );
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::colored(
                        AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                        &self.assets.guy.hat[&guy.skin.hat],
                        guy.skin.outfit_color,
                    ),
                );
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::colored(
                        AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                        &self.assets.guy.robe[&guy.skin.robe],
                        guy.skin.outfit_color,
                    ),
                );
                self.geng.draw_2d(
                    framebuffer,
                    &self.camera,
                    &geng::draw_2d::TexturedQuad::colored(
                        AABB::point(guy.position).extend_uniform(State::GUY_RADIUS),
                        &self.assets.guy.beard[&guy.skin.beard],
                        self.assets.constants.beard_color,
                    ),
                );
            }

            if let Some(pos) = self.camera.world_to_screen(
                self.framebuffer_size.map(|x| x as f32),
                guy.position + vec2(0.0, State::GUY_RADIUS),
            ) {
                let label_camera = if true {
                    self.camera.clone()
                } else {
                    geng::Camera2d {
                        center: Vec2::ZERO,
                        rotation: 0.0,
                        fov: 20.0_f32.max(self.camera.fov * 0.6),
                    }
                };
                let pos =
                    label_camera.screen_to_world(self.framebuffer_size.map(|x| x as f32), pos);

                let hp_bar_aabb =
                    AABB::point(pos + vec2(0.0, 0.2)).extend_symmetric(vec2(1.4, 0.2));
                self.geng.draw_2d(
                    framebuffer,
                    &label_camera,
                    &draw_2d::Quad::new(hp_bar_aabb.extend_uniform(0.1), Rgba::BLACK),
                );
                self.geng.draw_2d(
                    framebuffer,
                    &label_camera,
                    &draw_2d::Quad::new(hp_bar_aabb, Rgba::RED),
                );
                self.geng.draw_2d(
                    framebuffer,
                    &label_camera,
                    &draw_2d::Quad::new(
                        AABB {
                            x_max: hp_bar_aabb.x_min
                                + hp_bar_aabb.width() * guy.health as f32 / guy.max_health as f32,
                            ..hp_bar_aabb
                        },
                        Rgba::GREEN,
                    ),
                );

                if false {
                    let hp_text_aabb = AABB::point(pos + vec2(-1.5, 0.0)).extend_uniform(0.5);
                    self.geng.draw_2d(
                        framebuffer,
                        &label_camera,
                        &draw_2d::Quad::new(hp_text_aabb, Rgba::BLACK),
                    );
                    self.geng.draw_2d(
                        framebuffer,
                        &label_camera,
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
                } else {
                    self.geng.draw_2d(
                        framebuffer,
                        &label_camera,
                        &draw_2d::Text::unit(
                            &**self.geng.default_font(),
                            format!("lvl 5 ({}/{})", guy.health, guy.max_health),
                            Rgba::BLACK,
                        )
                        .fit_into(hp_bar_aabb.extend_uniform(-0.1)),
                    );
                }

                let name_aabb = AABB::point(pos + vec2(0.0, 0.8)).extend_symmetric(vec2(2.0, 0.2));
                self.geng.draw_2d(
                    framebuffer,
                    &label_camera,
                    &draw_2d::Text::unit(&**self.geng.default_font(), &guy.name, Rgba::BLACK)
                        .fit_into(name_aabb),
                );
            }
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
                    .scale_uniform(1.0 + t * effect.size * effect.scale_up)
                    .translate(effect.pos + vec2(0.0, -effect.offset)),
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
                    format!("type !{} to join", self.raffle_keyword),
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
                    "code&graphics - kuviman",
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
                    "music&sfx - BrainoidGames",
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
                sound_effect.set_volume(self.assets.constants.volume);
                sound_effect.play();
            }
            self.geng.draw_2d(
                framebuffer,
                &ui_camera,
                &font::Text::unit(
                    &self.geng,
                    &**self.geng.default_font(),
                    if winner.name == self.config.channel_login {
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
}
