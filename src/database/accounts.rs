use sqlx::{query, Error, Row};
use sqlx::mysql::MySqlRow;
use crate::database::DatabasePool;

pub struct Account {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub session: Option<String>
}

pub async fn create_account(pool: &DatabasePool, username: &str, password_hash: &str) -> Result<Account, Error> {
    let res = query("INSERT INTO Accounts (Username, Password) VALUES (?, ?) RETURNING ID, Username, Password, Session")
        .bind(username)
        .bind(password_hash)
        .fetch_one(pool.as_ref())
        .await;

    res.map(to_account)
}

pub async fn get_by_username(pool: &DatabasePool, username: &str) -> Result<Account, Error> {
    let res = query("SELECT * FROM Accounts WHERE Username = ?")
        .bind(username)
        .fetch_one(pool.as_ref())
        .await;

    res.map(to_account)
}

pub async fn set_session(pool: &DatabasePool, id: i32, session: Option<String>) -> Result<(), Error> {
    query("UPDATE Accounts SET Session = ? WHERE ID = ?")
        .bind(session)
        .bind(id)
        .execute(pool.as_ref())
        .await
        .map(|_| ())
}

fn to_account(row: MySqlRow) -> Account {
    Account {
        id: row.get(0),
        username: row.get(1),
        password_hash: row.get(2),
        session: row.get(3)
    }
}