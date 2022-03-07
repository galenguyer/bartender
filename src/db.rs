use sqlx::{Pool, Postgres};

pub mod models {
    #[derive(sqlx::FromRow, Debug)]
    pub struct Machine {
        pub id: i32,
        pub name: String,
        pub display_name: String,
        pub active: bool,
    }
    #[derive(sqlx::FromRow, Debug)]
    pub struct Slot {
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

pub async fn get_slots_with_items(pool: &Pool<Postgres>) -> Result<Vec<models::Slot>, sqlx::Error> {
    sqlx::query_as::<_, models::Slot>(
        "SELECT machine,number,item,active,count,id,name,price FROM slots 
        INNER JOIN items 
            ON slots.item = items.id 
        WHERE machine IN (
            SELECT id FROM machines 
                WHERE active = true
        )",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_active_machines(
    pool: &Pool<Postgres>,
) -> Result<Vec<models::Machine>, sqlx::Error> {
    sqlx::query_as::<_, models::Machine>("SELECT * FROM machines WHERE active = true")
        .fetch_all(pool)
        .await
}

pub async fn get_all_machines(pool: &Pool<Postgres>) -> Result<Vec<models::Machine>, sqlx::Error> {
    sqlx::query_as::<_, models::Machine>("SELECT * FROM machines")
        .fetch_all(pool)
        .await
}
