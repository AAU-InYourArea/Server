use std::sync::Arc;
use crate::data::{ConnectionData, Position};
use crate::error::{AnyErr, ProtocolError};

pub async fn set_frequency(connection_data: Arc<ConnectionData>, payload: serde_json::Value) -> Result<(), AnyErr> {
    // The payload is expected to be a number representing the new frequency
    // Frequency must be in the range of 0 to 255 (u8)
    let new_frequency = payload.as_u64().ok_or(ProtocolError::InvalidDataType)?;
    if new_frequency > u8::MAX as u64 {
        return Err(ProtocolError::InvalidDataType.into());
    }
    let new_frequency = new_frequency as u8;

    // Update the frequency in the connection data
    let mut frequency = connection_data.frequency.write().await;
    *frequency = new_frequency;

    Ok(())
}
pub async fn set_position(connection_data: Arc<ConnectionData>, payload: serde_json::Value) -> Result<(), AnyErr> {
    // The payload is expected to be a JSON object representing the new position
    let new_position: Position = serde_json::from_value(payload)?;

    // Update the position in the connection data
    let mut position = connection_data.position.write().await;
    *position = new_position;

    Ok(())
}