# agent-bridge Specification

## Purpose

TBD - created by archiving change 'rust-axum-agent-bridge'. Update Purpose after archive.

## Requirements

### Requirement: Session Creation
The system SHALL support creating a session, initializing it with a status of "starting" in SQLite, and transitioning it to "ready".

#### Scenario: Successful Session Creation
- **WHEN** the user sends a POST request to `/sessions`
- **THEN** the system SHALL return a HTTP 201 response with a newly created session ID, save the session to SQLite with state "starting", and transition it to "ready"

---
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


<!-- @trace
source: add-multimedia-and-human-intervention
updated: 2026-06-13
code:
  - src/frontend/index.js
  - src/infrastructure/runtime/daemon_client.rs
  - src/infrastructure/runtime/settings_ui.html
  - src/api/dto/message_dto.rs
  - src/frontend/index.html
  - src/infrastructure/runtime/daemon_settings.rs
  - src/infrastructure/runtime/mod.rs
  - src/application/models/session.rs
  - src/application/services/runtime_service.rs
  - src/infrastructure/runtime/gemini_cli.rs
  - src/application/ports/runtime_gateway.rs
  - src/frontend/style.css
  - daemon_config.json
  - src/infrastructure/db/message_repository_impl.rs
  - src/infrastructure/runtime/process_manager.rs
  - src/infrastructure/db/sqlite.rs
  - src/application/models/message.rs
  - Cargo.toml
  - src/api/routes/sessions.rs
  - src/application/services/session_service.rs
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
-->

---
### Requirement: Run Cancellation
The system SHALL support canceling a running Gemini CLI execution by tracking the PID of the child process on the host filesystem and sending a termination signal to its process group.

#### Scenario: Session Execution Cancelled
- **WHEN** the user sends a POST request to `/sessions/session_123/cancel` while a run is active
- **THEN** the Axum API SHALL call `POST /local/sessions/session_123/cancel` on the Daemon, and the Daemon SHALL read the PID from `/tmp/agent-pids/session_123.pid`, send a termination signal to the process tree, delete the PID file, and return success

---
### Requirement: SQLite Persistence
The system SHALL persist the final response message and update the session state in SQLite upon successful execution completion, without persisting intermediate token deltas.

#### Scenario: Persistence of completed run
- **WHEN** the Gemini CLI child process exits successfully with status code 0
- **THEN** the Axum API SHALL write the final complete message to the `messages` table in SQLite, update the session status to `ready`, and set `last_seen_at` and `expires_at`

---
### Requirement: Background Cleanup
The system API background task SHALL periodically scan SQLite for sessions that have exceeded their idle timeout or hard TTL, and for each expired session, send a DELETE command to the Daemon to clean up any running processes.

#### Scenario: Expired session cleanup
- **WHEN** the background cleanup task runs and finds a session with `expires_at` in the past
- **THEN** the API SHALL send a `DELETE /local/sessions/session_123` request to the Daemon, and the Daemon SHALL terminate any active child process, delete the PID file, and the API SHALL update the session status to `expired` in SQLite

##### Example: Cleanup Thresholds
- **GIVEN** a session created at `2026-06-12T22:50:04Z` with idle timeout of 30 minutes
- **WHEN** the current time is `2026-06-12T23:20:05Z`
- **THEN** the session SHALL be flagged as expired and any active process killed