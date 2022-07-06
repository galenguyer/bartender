use crate::db::models;
use sqlx::{Pool, Postgres};

pub async fn get_slots_with_items(
    pool: &Pool<Postgres>,
    machine: Option<i32>,
) -> Result<Vec<models::SlotWithItem>, sqlx::Error> {
    match machine {
        Some(machine_id) => {
            sqlx::query_as::<_, models::SlotWithItem>(
                "SELECT machine,number,item,active,count,id,name,price FROM slots 
                INNER JOIN items 
                    ON slots.item = items.id 
                WHERE machine = $1
                ORDER BY machine, number ASC",
            )
            .bind(machine_id)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, models::SlotWithItem>(
                "SELECT machine,number,item,active,count,id,name,price FROM slots 
                INNER JOIN items 
                    ON slots.item = items.id 
                WHERE machine IN (
                    SELECT id FROM machines 
                        WHERE active = true
                )
                ORDER BY machine, number ASC",
            )
            .fetch_all(pool)
            .await
        }
    }
}

pub async fn get_slot(
    pool: &Pool<Postgres>,
    machine_id: i32,
    slot_id: i32,
) -> Result<models::Slot, sqlx::Error> {
    sqlx::query_as::<_, models::Slot>(
        "SELECT machine,number,item,active,count FROM slots
        WHERE machine = $1 AND number = $2",
    )
    .bind(machine_id)
    .bind(slot_id)
    .fetch_one(pool)
    .await
}

pub async fn get_slot_with_item(
    pool: &Pool<Postgres>,
    machine_id: i32,
    slot: i32,
) -> Result<models::SlotWithItem, sqlx::Error> {
    sqlx::query_as::<_, models::SlotWithItem>(
        "SELECT machine,number,item,active,count,id,name,price FROM slots 
        INNER JOIN items 
            ON slots.item = items.id 
        WHERE machine = $1 AND number = $2",
    )
    .bind(machine_id)
    .bind(slot)
    .fetch_one(pool)
    .await
}

pub async fn update_slot_count(
    pool: &Pool<Postgres>,
    machine_id: i32,
    slot_id: i32,
    new_count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE slots 
                SET count = $1
                WHERE machine = $2 AND number = $3",
    )
    .bind(new_count)
    .bind(machine_id)
    .bind(slot_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_slot_active(
    pool: &Pool<Postgres>,
    machine_id: i32,
    slot_id: i32,
    active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE slots 
                SET active = $1
                WHERE machine = $2 AND number = $3",
    )
    .bind(active)
    .bind(machine_id)
    .bind(slot_id)
    .execute(pool)
    .await?;

    Ok(())
}
pub async fn update_slot_item(
    pool: &Pool<Postgres>,
    machine_id: i32,
    slot_id: i32,
    item_id: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE slots 
                SET item = $1
                WHERE machine = $2 AND number = $3",
    )
    .bind(item_id)
    .bind(machine_id)
    .bind(slot_id)
    .execute(pool)
    .await?;

    Ok(())
}
