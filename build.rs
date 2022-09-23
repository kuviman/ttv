/// https://docs.rs/sqlx/latest/sqlx/macro.migrate.html#triggering-recompilation-on-migration-changes

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
