use serde::Deserialize;
use crate::application::models::message::Attachment;

#[derive(Deserialize)]
pub struct PromptRequest {
    pub content: String,
    pub attachments: Option<Vec<Attachment>>,
}
