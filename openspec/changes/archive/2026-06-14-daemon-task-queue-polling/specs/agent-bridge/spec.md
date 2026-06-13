## MODIFIED Requirements

### Requirement: Real-time Message Streaming
The system SHALL support sending a prompt message to a session by writing a task to a database task queue, allowing the Local Agent Daemon to poll for the task, execute the CLI child process, and report progress tokens back to the API via HTTP, which are then pushed to the WebSocket client.

#### Scenario: Successful Prompt execution with SSE streaming and image attachments
- **WHEN** the user sends a POST request to `/sessions/session_123/messages` with a valid prompt and Base64-encoded image attachments
- **THEN** the API server SHALL write a task to the database task queue, return a HTTP 202 response, and stream the generated response back to the WebSocket client once the Local Agent Daemon polls the task, executes it, and sends progress updates back to the API

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
- **THEN** the task queue SHALL capture the payload, and the daemon SHALL poll the task, write the image to a temporary file, and pass it to the spawned CLI command.

### Requirement: Run Cancellation
The system SHALL support canceling a running Gemini CLI execution by updating the task status in the task queue or queueing a cancel task, allowing the Local Agent Daemon to detect the cancellation, terminate the child process tree on the host, and delete the PID file.

#### Scenario: Session Execution Cancelled
- **WHEN** the user sends a POST request to `/sessions/session_123/cancel` while a run is active
- **THEN** the Axum API SHALL update the task in the database task queue, and the Local Agent Daemon SHALL detect the cancellation during polling or progress reporting, send a termination signal to the process tree, delete the PID file, and return success
