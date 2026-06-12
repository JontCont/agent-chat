use serde::Deserialize;

#[derive(Deserialize)]
pub struct PromptRequest {
    pub content: String,
}
