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


<!-- @trace
source: daemon-task-queue-polling
updated: 2026-06-14
code:
  - README.zh-TW.md
  - src/infrastructure/runtime/daemon_client.rs
  - src/infrastructure/runtime/gemini_cli.rs
  - src/main.rs
  - README.md
  - docker-compose.yml
  - src/application/models/mod.rs
  - src/infrastructure/config/env.rs
  - run_api.ps1
  - src/api/routes/sessions.rs
  - src/infrastructure/db/sqlite.rs
  - src/application/models/task.rs
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
  - .env.example
  - run_daemon.ps1
  - Dockerfile
  - run_tests.ps1
  - build_release.ps1
  - src/api/mod.rs
-->

---
### Requirement: Run Cancellation
The system SHALL support canceling a running Gemini CLI execution by updating the task status in the task queue or queueing a cancel task, allowing the Local Agent Daemon to detect the cancellation, terminate the child process tree on the host, and delete the PID file.

#### Scenario: Session Execution Cancelled
- **WHEN** the user sends a POST request to `/sessions/session_123/cancel` while a run is active
- **THEN** the Axum API SHALL update the task in the database task queue, and the Local Agent Daemon SHALL detect the cancellation during polling or progress reporting, send a termination signal to the process tree, delete the PID file, and return success


<!-- @trace
source: daemon-task-queue-polling
updated: 2026-06-14
code:
  - README.zh-TW.md
  - src/infrastructure/runtime/daemon_client.rs
  - src/infrastructure/runtime/gemini_cli.rs
  - src/main.rs
  - README.md
  - docker-compose.yml
  - src/application/models/mod.rs
  - src/infrastructure/config/env.rs
  - run_api.ps1
  - src/api/routes/sessions.rs
  - src/infrastructure/db/sqlite.rs
  - src/application/models/task.rs
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
  - .env.example
  - run_daemon.ps1
  - Dockerfile
  - run_tests.ps1
  - build_release.ps1
  - src/api/mod.rs
-->

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