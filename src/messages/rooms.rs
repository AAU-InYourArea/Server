use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateChatroom {
    pub name: String,
    pub password: String
}

#[derive(Deserialize)]
pub struct JoinChatroom {
    pub room: i32,
    pub password: String
}