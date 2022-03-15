use crate::db::models;
use sqlx::{Pool, Postgres};

pub async fn get_active_machines(
    pool: &Pool<Postgres>,
) -> Result<Vec<models::Machine>, sqlx::Error> {
    sqlx::query_as::<_, models::Machine>(
        "SELECT * FROM machines WHERE active = true ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_all_machines(pool: &Pool<Postgres>) -> Result<Vec<models::Machine>, sqlx::Error> {
    sqlx::query_as::<_, models::Machine>("SELECT * FROM machines ORDER BY id ASC")
        .fetch_all(pool)
        .await
}

pub async fn get_machine(
    pool: &Pool<Postgres>,
    name: &str,
) -> Result<models::Machine, sqlx::Error> {
    sqlx::query_as::<_, models::Machine>("SELECT * FROM machines WHERE active = true AND name = $1")
        .bind(name)
        .fetch_one(pool)
        .await
}
