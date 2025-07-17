use sqlx::mysql::MySqlRow;
use sqlx::{Error, Row};
use crate::database::DatabasePool;

pub struct Room {
    pub id: i32,
    pub name: String,
    pub password_hash: String
}

pub async fn create_room(database_pool: &DatabasePool, name: &str, password_hash: &str) -> Result<i32, Error> {
    let res = sqlx::query("INSERT INTO Rooms (Name, Password) VALUES (?, ?) RETURNING ID")
        .bind(name)
        .bind(password_hash)
        .fetch_one(database_pool.as_ref())
        .await?;

    Ok(res.get(0))
}

pub async fn delete_room(database_pool: &DatabasePool, id: i32) -> Result<(), Error> {
    sqlx::query("DELETE FROM Rooms WHERE ID = ?")
        .bind(id)
        .execute(database_pool.as_ref())
        .await
        .map(|_| ())
}

pub async fn get_room(database_pool: &DatabasePool, id: i32) -> Result<Room, Error> {
    let res = sqlx::query("SELECT * FROM Rooms WHERE ID = ?")
        .bind(id)
        .fetch_one(database_pool.as_ref())
        .await;

    res.map(to_room)
}

pub async fn get_rooms(database_pool: &DatabasePool) -> Result<Vec<Room>, Error> {
    let res = sqlx::query("SELECT * FROM Rooms WHERE ID > ?")
        .bind(0)
        .fetch_all(database_pool.as_ref())
        .await;

    res.map(|rows| rows.into_iter().map(to_room).collect())
}

fn to_room(row: MySqlRow) -> Room {
    Room {
        id: row.get(0),
        name: row.get(1),
        password_hash: row.get(2)
    }
}