use serde::Deserialize;

/// generic top-level request structure for direct requests
#[derive(Deserialize)]
pub struct DirectRequest {
    #[serde(rename = "commandId")]
    pub command_id: usize,
    pub r#type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}