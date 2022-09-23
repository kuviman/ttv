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
            self.set_level(name, 0);
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
                sqlx::query("INSERT INTO `Guy` VALUES (?, ?)")
                    .bind(name)
                    .bind(level as i64)
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
    assert!(db.find_level("kuviman") == 0);
    db.set_level("kuviman", 5);
    assert!(db.find_level("kuviman") == 5);
}
