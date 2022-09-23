use sqlx::{ConnectOptions, Executor};

use super::*;

pub struct Db {
    connection: sqlx::SqliteConnection,
}

impl Db {
    pub fn new(url: &str) -> Self {
        block_on(async {
            let mut connection = url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()
                .unwrap()
                .create_if_missing(true)
                .connect()
                .await
                .unwrap();
            connection.execute(include_str!("setup.sql")).await.unwrap();
            Self { connection }
        })
    }
    pub fn find_level(&mut self, name: &str) -> usize {
        let result: Option<(i64,)> = block_on(
            sqlx::query_as("SELECT level from Persons WHERE name=?")
                .bind(name)
                .fetch_optional(&mut self.connection),
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
            if sqlx::query("UPDATE Persons SET level=? WHERE name=?")
                .bind(level as i64)
                .bind(name)
                .execute(&mut self.connection)
                .await
                .unwrap()
                .rows_affected()
                == 0
            {
                sqlx::query("INSERT INTO Persons VALUES (?, ?)")
                    .bind(name)
                    .bind(level as i64)
                    .execute(&mut self.connection)
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
