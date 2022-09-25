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
    pub fn find_level(&mut self, name: &str, insert_if_absent: bool) -> usize {
        let result: Option<(i64,)> = block_on(
            sqlx::query_as("SELECT `level` FROM `Guy` WHERE `name`=?")
                .bind(name)
                .fetch_optional(&self.pool),
        )
        .unwrap();
        if let Some((level,)) = result {
            level as usize
        } else {
            if insert_if_absent {
                self.set_level(name, 0);
            }
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
}

#[test]
fn test_db() {
    logger::init_for_tests();
    let mut db = Db::new("sqlite::memory:");
    assert!(db.find_level("kuviman", true) == 0);
    db.set_level("kuviman", 5);
    assert!(db.find_level("kuviman", true) == 5);
    db.set_game_link("kuviman", Some("123"));
    db.set_game_link("kuviman", None);
    db.set_game_link("random_dude", Some("123"));
    db.set_game_link("random_dude2", None);
}
