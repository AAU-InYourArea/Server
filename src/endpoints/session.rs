use std::sync::Arc;
use crate::data::{ConnectionData, GlobalData};
use crate::database::accounts::set_session;
use crate::error::{AnyErr, ProtocolError};

pub async fn logout(global_data: Arc<GlobalData>, connection_data: Arc<ConnectionData>) -> Result<(), AnyErr> {
    let mut account = connection_data.account.write().await;
    account.session = None;
    set_session(&global_data.database_pool, account.id, None).await?;
    Err(ProtocolError::LoggedOut.into())
}