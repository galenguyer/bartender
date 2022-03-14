use sqlx::{Pool, Postgres};

pub mod models {
    use serde::Serialize;

    #[derive(sqlx::FromRow, Debug, Serialize)]
    pub struct Machine {
        pub id: i32,
        pub name: String,
        pub display_name: String,
        pub active: bool,
    }
    #[derive(sqlx::FromRow, Debug, Serialize)]
    pub struct Slot {
        pub machine: i32,
        pub number: i32,
        pub item: i32,
        pub active: bool,
        pub count: Option<i32>,
    }
    #[derive(sqlx::FromRow, Debug, Serialize)]
    pub struct Item {
        pub id: i32,
        pub name: String,
        pub price: i32,
    }
    #[derive(sqlx::FromRow, Debug, Serialize)]
    pub struct SlotWithItem {
        pub machine: i32,
        pub number: i32,
        pub item: i32,
        pub active: bool,
        pub count: Option<i32>,
        pub id: i32,
        pub name: String,
        pub price: i32,
    }
}

pub async fn get_slots_with_items(
    pool: &Pool<Postgres>,
) -> Result<Vec<models::SlotWithItem>, sqlx::Error> {
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

pub async fn get_item(pool: &Pool<Postgres>, item_id: i32) -> Result<models::Item, sqlx::Error> {
    sqlx::query_as::<_, models::Item>(
        "SELECT id,name,price FROM items
        WHERE id = $1",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
}

pub async fn get_items(pool: &Pool<Postgres>) -> Result<Vec<models::Item>, sqlx::Error> {
    sqlx::query_as::<_, models::Item>(
        "SELECT id,name,price FROM items",
    )
    .fetch_all(pool)
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
    let _ = sqlx::query(
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
    let _ = sqlx::query(
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
    let _ = sqlx::query(
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
