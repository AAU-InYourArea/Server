use std::sync::Arc;
use crate::check_chatroom_empty;
use crate::data::{ConnectionData, GlobalData};
use crate::database::rooms::{create_room, get_room, get_rooms};
use crate::endpoints::answer;
use crate::error::AnyErr;
use crate::hash::{hash, verify};
use crate::messages::rooms::{CreateChatroom, JoinChatroom, RoomResponse};

pub async fn create_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, payload: CreateChatroom, command_id: usize) -> Result<(), AnyErr> {
    let room = create_room(&global_data.database_pool, &payload.name, hash(&payload.password)?.as_str()).await?;
    { // this connection automatically joins the room
        let mut current_room = connection_data.room.write().await;
        *current_room = Some(room);
    }

    // send the room id back to the client
    answer(connection_data, command_id, room).await?;
    Ok(())
}

pub async fn join_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, payload: JoinChatroom, command_id: usize) -> Result<(), AnyErr> {
    let room = get_room(&global_data.database_pool, payload.room).await?;
    if verify(&payload.password, &room.password_hash)? { // check if the provided password matches
        { // this connection joins the room
            let mut current_room = connection_data.room.write().await;
            *current_room = Some(room.id);
        }
        
        // send the room id back to the client
        answer(connection_data, command_id, room.id).await?;
    } else {
        // if the password is incorrect, send an error response (room id -1)
        answer(connection_data, command_id, -1).await?;
    }

    Ok(())
}

pub async fn get_rooms_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, command_id: usize) -> Result<(), AnyErr> {
    // get all rooms from the database
    let rooms = get_rooms(&global_data.database_pool).await?;
    
    // map the rooms to the response format
    let rooms: Vec<RoomResponse> = rooms.into_iter().map(|r| {
        RoomResponse {
            id: r.id,
            name: r.name
        }
    }).collect();
    
    // send the rooms back to the client
    answer(connection_data, command_id, rooms).await?;
    Ok(())
}

pub async fn leave_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>) -> Result<(), AnyErr> {
    let room_id = { // clear the current room for this connection
        let mut current_room = connection_data.room.write().await;
        let room_id = (*current_room).clone();
        *current_room = None;
        room_id
    };
    if let Some(room_id) = room_id { // if the connection was in a room, check if it is empty now
        check_chatroom_empty(&global_data, room_id).await?;
    }
    Ok(())
}