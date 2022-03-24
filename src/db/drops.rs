use crate::db::models;
use sqlx::{Pool, Postgres};

pub async fn log_drop(pool: &Pool<Postgres>, drop: &models::Drop) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO drops(username, machine, slot, item, item_name, item_price) 
        VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&drop.username)
    .bind(drop.machine)
    .bind(drop.slot)
    .bind(drop.item)
    .bind(&drop.item_name)
    .bind(drop.item_price)
    .execute(pool)
    .await?;

    Ok(())
}
