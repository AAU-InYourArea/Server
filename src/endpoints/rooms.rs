use std::sync::Arc;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::check_chatroom_empty;
use crate::data::{ConnectionData, GlobalData};
use crate::database::rooms::{create_room, get_room};
use crate::endpoints::answer;
use crate::error::AnyErr;
use crate::hash::verify;
use crate::messages::rooms::{CreateChatroom, JoinChatroom};

pub async fn create_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, payload: CreateChatroom, command_id: usize) -> Result<(), AnyErr> {
    let room = create_room(&global_data.database_pool, &payload.name, &payload.password).await?;
    {
        let mut current_room = connection_data.room.write().await;
        *current_room = Some(room.id);
    }

    answer(connection_data, command_id, room.id).await?;
    Ok(())
}

pub async fn join_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, payload: JoinChatroom, command_id: usize) -> Result<(), AnyErr> {
    let room = get_room(&global_data.database_pool, payload.room).await?;
    if verify(&payload.password, &room.password_hash)? {
        {
            let mut current_room = connection_data.room.write().await;
            *current_room = Some(room.id);
        }
        answer(connection_data, command_id, room.id).await?;
    } else {
        answer(connection_data, command_id, -1).await?;
    }

    Ok(())
}

pub async fn leave_room_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>) -> Result<(), AnyErr> {
    {
        let mut current_room = connection_data.room.write().await;
        if let Some(room_id) = *current_room {
            check_chatroom_empty(&global_data, room_id).await?;
        }
        *current_room = None;
    }
    Ok(())
}