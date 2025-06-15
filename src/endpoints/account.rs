use std::sync::Arc;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::data::ConnectionData;
use crate::error::AnyErr;
use crate::messages::login::LoginResponse;

pub async fn account(connection_data: Arc<ConnectionData>, command_id: usize) -> Result<(), AnyErr> {
    let response = {
        let account = connection_data.account.read().await;
        LoginResponse {
            success: true,
            username: Some(account.username.clone()),
            session: account.session.clone(),
        }
    };

    let msg = serde_json::to_string(&response)?;
    connection_data.channel.send(Message::Text(Utf8Bytes::from(format!("{} {}", command_id, msg)))).await?;
    Ok(())
}