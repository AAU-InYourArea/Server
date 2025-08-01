use crate::data::ConnectionData;
use crate::error::AnyErr;
use crate::messages::login::LoginResponse;
use std::sync::Arc;
use crate::endpoints::answer;

pub async fn account(connection_data: Arc<ConnectionData>, command_id: usize) -> Result<(), AnyErr> {
    let response = { // construct a login response
        let account = connection_data.account.read().await;
        LoginResponse {
            success: true,
            username: Some(account.username.clone()),
            session: account.session.clone(),
        }
    };

    // send the response back to the client
    answer(connection_data, command_id, response).await?;
    Ok(())
}