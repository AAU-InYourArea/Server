mod account;
mod session;
mod data_set;
mod rooms;

use crate::data::{ConnectionData, GlobalData};
use crate::endpoints::account::account;
use crate::endpoints::data_set::{set_frequency, set_position};
use crate::endpoints::session::logout;
use crate::error::AnyErr;
use crate::messages::direct_request::DirectRequest;
use serde::Serialize;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use bytes::Bytes;
use crate::endpoints::rooms::{create_room_request, get_rooms_request, join_room_request, leave_room_request};

/// handle a websocket message from the client
pub async fn direct_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, message: Message) -> Result<(), AnyErr> {
    if message.is_text() { // text messages are treated as commands
        let request = message.into_text()?;
        let request: DirectRequest = serde_json::from_str(request.as_str())?;

        match request.r#type.as_str() {
            "account" => account(connection_data, request.command_id).await,
            "logout" => logout(global_data, connection_data).await,
            "frequency" => set_frequency(connection_data, request.payload).await,
            "position" => set_position(connection_data, request.payload).await,
            "room_create" => create_room_request(global_data, connection_data, serde_json::from_value(request.payload)?, request.command_id).await,
            "room_join" => join_room_request(global_data, connection_data, serde_json::from_value(request.payload)?, request.command_id).await,
            "room_leave" => leave_room_request(global_data, connection_data).await,
            "rooms" => get_rooms_request(global_data, connection_data, request.command_id).await,
            _ => Ok(())
        }
    } else if message.is_binary() { // binary messages are assumed to be voice data
        let username = { // get the username from the connection data
            let account = connection_data.account.read().await;
            account.username.clone()
        };
        let data = message.into_data();
        // add a byte in front with the length of the username
        // add the username as well
        let mut data_with_sender = vec![];
        data_with_sender.push(username.len() as u8);
        data_with_sender.extend_from_slice(username.as_bytes());
        data_with_sender.extend_from_slice(data.as_ref());
        let message = Message::Binary(Bytes::from(data_with_sender));

        // send the message to all connections that can hear this connection
        let broadcast = connection_data.broadcast.read().await;
        for conn in global_data.connections.read().await.values() {
            if broadcast.contains(&conn.id) {
                let msg = message.clone();
                conn.channel.send(msg).await?;
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

async fn answer<T: Serialize>(connection_data: Arc<ConnectionData>, command_id: usize, msg: T) -> Result<(), AnyErr> {
    let msg = serde_json::to_string(&msg)?;
    connection_data.channel.send(Message::Text(Utf8Bytes::from(format!("{} {}", command_id, msg)))).await?;
    Ok(())
}