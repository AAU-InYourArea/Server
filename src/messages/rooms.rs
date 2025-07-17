use serde::{Deserialize, Serialize};

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

#[derive(Serialize)]
pub struct RoomResponse {
    pub id: i32,
    pub name: String
}