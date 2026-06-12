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
The system SHALL support sending a prompt message to a session, spawning a Gemini CLI child process on the host via the Local Agent Daemon, and streaming tokens back via HTTP Server-Sent Events (SSE) which are then pushed to the WebSocket client.

#### Scenario: Successful Prompt execution with SSE streaming
- **WHEN** the user sends a POST request to `/sessions/session_123/messages` with a valid prompt, and connects to the WebSocket endpoint `/ws/session_123`
- **THEN** the Local Agent Daemon SHALL spawn a Gemini CLI child process, pipe its stdout/stderr, stream delta events via HTTP SSE, and the Axum API SHALL forward those delta events to the WebSocket client as JSON frames

##### Example: Stream Events
- **GIVEN** a prompt message "Hello"
- **WHEN** the Gemini CLI output contains "Hello!"
- **THEN** the SSE stream SHALL yield:
  | Event | Data | Notes |
  | --- | --- | --- |
  | `delta` | `{"text": "Hello"}` | First token chunk |
  | `delta` | `{"text": "!"}` | Second token chunk |
  | `done` | `{"text": "Hello!"}` | Final complete message |

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
