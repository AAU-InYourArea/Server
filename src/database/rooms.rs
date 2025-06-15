use sqlx::mysql::MySqlRow;
use sqlx::{Error, Row};
use crate::database::DatabasePool;

pub struct Room {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub password_hash: String,
    pub creator: i32
}

pub async fn create_room(database_pool: &DatabasePool, name: &str, description: &str, password_hash: &str, creator: i32) -> Result<Room, Error> {
    let res = sqlx::query("INSERT INTO Rooms (Name, Description, Password, Creator) VALUES (?, ?, ?, ?) RETURNING ID, Name, Description, Password, Creator")
        .bind(name)
        .bind(description)
        .bind(password_hash)
        .bind(creator)
        .fetch_one(database_pool.as_ref())
        .await;

    res.map(to_room)
}

pub async fn get_room(database_pool: &DatabasePool, id: i32) -> Result<Room, Error> {
    let res = sqlx::query("SELECT * FROM Rooms WHERE ID = ?")
        .bind(id)
        .fetch_one(database_pool.as_ref())
        .await;

    res.map(to_room)
}

pub async fn get_rooms(database_pool: &DatabasePool) -> Result<Vec<Room>, Error> {
    let res = sqlx::query("SELECT * FROM Rooms")
        .fetch_all(database_pool.as_ref())
        .await;

    res.map(|rows| rows.into_iter().map(to_room).collect())
}

fn to_room(row: MySqlRow) -> Room {
    Room {
        id: row.get(0),
        name: row.get(1),
        description: row.get(2),
        password_hash: row.get(3),
        creator: row.get(4)
    }
}