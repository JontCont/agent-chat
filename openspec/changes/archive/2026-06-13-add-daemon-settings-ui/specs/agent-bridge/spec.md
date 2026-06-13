## MODIFIED Requirements

### Requirement: Real-time Message Streaming
The system SHALL support sending a prompt message to a session, spawning the configured CLI child process on the host via the Local Agent Daemon, and streaming tokens back via HTTP Server-Sent Events (SSE) which are then pushed to the WebSocket client.

#### Scenario: Successful Prompt execution with SSE streaming
- **WHEN** the user sends a POST request to `/sessions/session_123/messages` with a valid prompt, and connects to the WebSocket endpoint `/ws/session_123`
- **THEN** the Local Agent Daemon SHALL spawn the configured CLI child process, pipe its stdout/stderr, stream delta events via HTTP SSE, and the Axum API SHALL forward those delta events to the WebSocket client as JSON frames

##### Example: Stream Events
- **GIVEN** a prompt message "Hello"
- **WHEN** the configured CLI output contains "Hello!"
- **THEN** the SSE stream SHALL yield:
  | Event | Data | Notes |
  | --- | --- | --- |
  | `delta` | `{"text": "Hello"}` | First token chunk |
  | `delta` | `{"text": "!"}` | Second token chunk |
  | `done` | `{"text": "Hello!"}` | Final complete message |
