mod account;
mod session;
mod data_set;

use crate::data::{ConnectionData, GlobalData};
use crate::endpoints::account::account;
use crate::endpoints::data_set::{set_frequency, set_position};
use crate::endpoints::session::logout;
use crate::error::AnyErr;
use crate::messages::direct_request::DirectRequest;
use serde::Serialize;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

pub async fn direct_request(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>, message: Message) -> Result<(), AnyErr> {
    if message.is_text() {
        let request = message.into_text()?;
        let request: DirectRequest = serde_json::from_str(request.as_str())?;

        match request.r#type.as_str() {
            "account" => account(connection_data, request.command_id).await,
            "logout" => logout(global_data, connection_data).await,
            "frequency" => set_frequency(connection_data, request.payload).await,
            "position" => set_position(connection_data, request.payload).await,
            _ => Ok(())
        }
    } else if message.is_binary() {
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

async fn answer<T: Serialize>(connection_data: Arc<ConnectionData>, msg: T) -> Result<(), AnyErr> {
    let msg = serde_json::to_string(&msg)?;
    connection_data.channel.send(Message::Text(Utf8Bytes::from(msg))).await?;
    Ok(())
}