## MODIFIED Requirements

### Requirement: Message Streaming and Input Locking
The system SHALL allow users to enter prompt text and attach local images. When the user sends a prompt, the system SHALL encode any attached images in Base64 and send a POST request to `/sessions/{session_id}/messages` containing the prompt and the image attachments. The system SHALL disable input fields and the Send button during streaming, and update the chat area in real-time with response deltas.

#### Scenario: Stream response deltas with image attachment
- **WHEN** the user attaches an image and submits a prompt
- **THEN** the system SHALL preview the image, convert it to Base64, send a POST request with the prompt text and Base64 attachment, disable inputs, and update the chat area with stream deltas
