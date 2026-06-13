## MODIFIED Requirements

### Requirement: Real-time Message Streaming
The system SHALL support sending a prompt message to a session with optional Base64 image attachments, decoding the Base64 images to local temporary files on the host, executing the CLI child process, and streaming tokens back via HTTP Server-Sent Events (SSE) which are then pushed to the WebSocket client.

#### Scenario: Successful Prompt execution with SSE streaming and image attachments
- **WHEN** the user sends a POST request to `/sessions/session_123/messages` with a valid prompt and Base64-encoded image attachments, and connects to the WebSocket endpoint `/ws/session_123`
- **THEN** the Local Agent Daemon SHALL decode the Base64 image data to a temporary file on the host machine, spawn the CLI child process passing the prompt and temporary file path, and stream the generated response back to the client

##### Example: Message Payload with Base64 Image
- **WHEN** the user sends prompt "What is this?" with a PNG image:
  ```json
  {
    "content": "What is this?",
    "attachments": [
      {
        "mime_type": "image/png",
        "data": "iVBORw0KGgoAAAANSUhEUgAA..."
      }
    ]
  }
  ```
- **THEN** the daemon SHALL write it to a temporary file and pass it to the spawned CLI command.
