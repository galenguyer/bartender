use crate::db::models;
use sqlx::{Pool, Postgres};

pub async fn get_items(pool: &Pool<Postgres>) -> Result<Vec<models::Item>, sqlx::Error> {
    sqlx::query_as::<_, models::Item>("SELECT id,name,price FROM items")
        .fetch_all(pool)
        .await
}

pub async fn get_item(pool: &Pool<Postgres>, item_id: i32) -> Result<models::Item, sqlx::Error> {
    sqlx::query_as::<_, models::Item>(
        "SELECT id,name,price FROM items
        WHERE id = $1",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
}

pub async fn create_item(pool: &Pool<Postgres>, name: &str, price: i32) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO items(name, price) VALUES ($1, $2)")
        .bind(name)
        .bind(price)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_item_name(
    pool: &Pool<Postgres>,
    id: i32,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE items SET name = $1 WHERE id = $2")
        .bind(name)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_item_price(
    pool: &Pool<Postgres>,
    id: i32,
    price: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE items SET price = $1 WHERE id = $2")
        .bind(price)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_item(pool: &Pool<Postgres>, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM items WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}
