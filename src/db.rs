use super::*;

pub struct Db {
    pool: sqlx::AnyPool,

    // Idle connection is there for sqlite::memory: db
    // Otherwise db is wiped when all connections get closed
    #[allow(dead_code)]
    idle_connection: sqlx::pool::PoolConnection<sqlx::Any>,
}

impl Db {
    /// Connect to db
    ///
    /// Example urls:
    /// - `sqlite::memory:`
    /// - `sqlite://data.db?mode=rwc`
    pub fn new(url: &str) -> Self {
        block_on(async {
            let pool = sqlx::AnyPool::connect(url)
                .await
                .expect("Failed to connect to database");
            let idle_connection = pool.acquire().await.unwrap();
            sqlx::migrate!()
                .run(&pool)
                .await
                .expect("Failed to run migrations");
            Self {
                pool,
                idle_connection,
            }
        })
    }
    pub fn find_level(&mut self, name: &str) -> usize {
        let result: Option<(i64,)> = block_on(
            sqlx::query_as("SELECT `level` FROM `Guy` WHERE `name`=?")
                .bind(name)
                .fetch_optional(&self.pool),
        )
        .unwrap();
        if let Some((level,)) = result {
            level as usize
        } else {
            0
        }
    }
    pub fn set_level(&mut self, name: &str, level: usize) {
        block_on(async {
            if sqlx::query("UPDATE `Guy` SET `level`=? WHERE `name`=?")
                .bind(level as i64)
                .bind(name)
                .execute(&self.pool)
                .await
                .unwrap()
                .rows_affected()
                == 0
            {
                sqlx::query("INSERT INTO `Guy` (`name`, `level`) VALUES (?, ?)")
                    .bind(name)
                    .bind(level as i64)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
        });
    }
    pub fn set_game_link(&mut self, name: &str, link: Option<&str>) {
        block_on(async {
            if sqlx::query("UPDATE `Guy` SET `game_link`=? WHERE `name`=?")
                .bind(link)
                .bind(name)
                .execute(&self.pool)
                .await
                .unwrap()
                .rows_affected()
                == 0
            {
                sqlx::query("INSERT INTO `Guy` (`name`, `game_link`) VALUES (?, ?)")
                    .bind(name)
                    .bind(link)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
        });
    }

    pub fn find_game_link(&self, name: &str) -> Option<String> {
        block_on(async {
            let result: Option<(Option<String>,)> =
                sqlx::query_as("SELECT `game_link` FROM `Guy` WHERE `name`=?")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap();
            result.and_then(|(url,)| url)
        })
    }

    pub fn game_played(&self, name: &str) -> bool {
        block_on(async {
            let result: Option<(bool,)> =
                sqlx::query_as("SELECT `game_played` FROM `Guy` WHERE `name`=?")
                    .bind(name)
                    .fetch_optional(&self.pool)
                    .await
                    .unwrap();
            result.map_or(false, |(played,)| played)
        })
    }
    pub fn set_game_played(&self, name: &str, played: bool) {
        block_on(async {
            assert!(
                sqlx::query("UPDATE `Guy` SET `game_played`=? WHERE `name`=?")
                    .bind(played)
                    .bind(name)
                    .execute(&self.pool)
                    .await
                    .unwrap()
                    .rows_affected()
                    == 1
            );
        });
    }
    pub fn enforce_guy_exists(&self, name: &str) {
        block_on(async {
            if sqlx::query("SELECT `name` FROM `Guy` WHERE `name`=?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .unwrap()
                .is_none()
            {
                sqlx::query("INSERT INTO `Guy` (`name`) VALUES (?)")
                    .bind(name)
                    .execute(&self.pool)
                    .await
                    .unwrap();
            }
        });
    }
    pub fn find_skin(&self, name: &str) -> Option<Skin> {
        block_on(async {
            if sqlx::query_as::<_, (Option<String>,)>("SELECT `face` FROM `Guy` WHERE `name`=?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await
                .unwrap()
                .and_then(|(face,)| face)
                .is_none()
            {
                return None;
            }
            Some(
                sqlx::query_as(
                    "SELECT
                    `face`,
                    `hat`,
                    `robe`,
                    `beard`,
                    `custom_skin`,
                    `outfit_color` FROM `Guy`
                    WHERE `name`=?",
                )
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .unwrap(),
            )
        })
    }
    pub fn set_skin(&self, name: &str, skin: &Skin) {
        self.enforce_guy_exists(name);
        block_on(async {
            let rows_affected = sqlx::query(
                "UPDATE `Guy` SET
                `face`=?,
                `hat`=?,
                `robe`=?,
                `beard`=?,
                `custom_skin`=?,
                `outfit_color`=?
                WHERE `name`=?",
            )
            .bind(&skin.face)
            .bind(&skin.hat)
            .bind(&skin.robe)
            .bind(&skin.beard)
            .bind(&skin.custom)
            .bind(skin.outfit_color.to_string())
            .bind(name)
            .execute(&self.pool)
            .await
            .unwrap()
            .rows_affected();
            assert_eq!(rows_affected, 1);
        });
    }
}

#[test]
fn test_db() {
    logger::init_for_tests();
    let mut db = Db::new("sqlite::memory:");
    assert!(db.find_level("kuviman") == 0);
    db.set_level("kuviman", 5);
    assert!(db.find_level("kuviman") == 5);
    db.set_game_link("kuviman", Some("123"));
    db.set_game_link("kuviman", None);
    db.set_game_link("random_dude", Some("123"));
    db.set_game_link("random_dude2", None);
    assert_eq!(db.find_game_link("random_dude").as_deref(), Some("123"));
    assert_eq!(db.find_game_link("random_dude2").as_deref(), None);
    assert_eq!(db.find_game_link("kuviman").as_deref(), None);
    assert_eq!(db.find_game_link("non_existent_dude").as_deref(), None);
    db.set_game_played("kuviman", true);
    assert!(db.game_played("kuviman"));
    assert!(!db.game_played("non_existent_dude"));
    assert!(!db.game_played("random_dude"));

    let skin = Skin {
        face: "face".to_owned(),
        hat: "hat".to_owned(),
        robe: "robe".to_owned(),
        beard: "beard".to_owned(),
        custom: None,
        outfit_color: Rgba::YELLOW,
    };
    assert_eq!(db.find_skin("kuviman"), None);
    db.set_skin("kuviman", &skin);
    db.set_skin("someone", &skin);
    assert_eq!(db.find_skin("kuviman"), Some(skin));
}
