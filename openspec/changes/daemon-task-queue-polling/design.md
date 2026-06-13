## Context

Currently, the API server sends HTTP requests to the Local Agent Daemon at `DAEMON_URL` (usually `http://<DAEMON_PC_IP>:7456`). If the Local PC is behind a NAT router or firewall, the API (which might be hosted on a NAS or cloud) cannot reach it.

## Goals / Non-Goals

**Goals:**
- Shift the API-Daemon communication to a polling-based model where the Daemon pulls tasks from the API.
- Keep the local Daemon UI accessible on `localhost`.
- Eliminate the need for `DAEMON_URL` configuration on the API server.

**Non-Goals:**
- We are not changing the SQLite database type (keeping SQLite).
- We are not changing the WebSocket flow between the frontend client and the API server.

## Decisions

### 1. Decision: Use SQLite Tasks Table for Task Queueing
- **Rationale**: Instead of installing a new message queue like RabbitMQ or Redis, we will use a `tasks` SQLite table in the existing SQLite database. This keeps dependencies minimal and matches the current WAL-mode SQLite database setup.

### 2. Decision: Expose Task Polling and Progress Endpoints on Axum API
- **Rationale**: We will expose `GET /sessions/tasks/poll` and `POST /sessions/tasks/:task_id/progress` endpoints. The Daemon will make outbound HTTP calls to these endpoints.

### 3. Decision: In-Memory MPSC Channel Registry for SSE Token Streaming
- **Rationale**: The `RuntimeGateway` interface expects a `Stream` of SSE lines. By registering a `mpsc::UnboundedSender` for each active task in a shared `TaskStreamRegistry`, we can feed the stream from the `/progress` endpoint. This allows us to keep the existing `RuntimeService` stream processing pipeline unchanged.

### 4. Decision: Daemon Loop to Bind to Localhost and Poll API
- **Rationale**: The Daemon will bind its HTTP server to `127.0.0.1` instead of `0.0.0.0` for local settings UI access, and it will spawn a background Tokio task to poll the API server at `BRIDGE_URL` (passed as an environment variable or argument) every 1 second.

## Implementation Contract

- **Behavior**:
  - A user sends a prompt message. The session status becomes `Busy`, and a `run` task is inserted.
  - The Daemon polls, receives the `run` task, executes the CLI process, and posts stdout lines back.
  - The frontend UI displays the streaming tokens in real-time.
  - If the user cancels the session, the task status is updated to `Cancelled` or a cancel task is inserted. The Daemon detects this, terminates the process, and reports success.
- **Interface / Data Shape**:
  - New DB Table `tasks`:
    ```sql
    CREATE TABLE tasks (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        task_type TEXT NOT NULL,
        payload TEXT, -- JSON containing content, attachments, active_cli
        status TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    ```
  - Endpoints:
    - `GET /sessions/tasks/poll`
      Returns: JSON list of pending tasks, e.g.:
      `[{"id": "...", "session_id": "...", "task_type": "run", "payload": "..."}]`
    - `POST /sessions/tasks/:task_id/progress`
      Payload: `{"line": "..."}`
      Returns: `{"continue": true}` (or `false` if cancelled)
    - `POST /sessions/tasks/:task_id/complete`
      Payload: `{"status": "completed"|"failed", "error": null|string}`
- **Failure Modes**:
  - If the Daemon fails to connect to the API server during polling, it will retry after 5 seconds.
  - If a task execution fails on the Daemon, it reports the error via `/complete`, and the API marks the session as `Faulted`.
- **Acceptance Criteria**:
  - The user can run prompts and see real-time streaming updates without the API having access to the Daemon's port.
  - Session cancellation terminates the daemon execution process group correctly.
- **Scope Boundaries**:
  - In Scope: Implementing the SQLite task queue, exposing API endpoints, modifying Daemon to poll, and restricting Local Daemon UI to localhost.
  - Out of Scope: Real-time notification of Daemon availability (it will only be checked via polling).

## Risks / Trade-offs

- **[Risk] Poll Latency** $\rightarrow$ Polling every 1 second introduces up to 1 second delay before execution starts.
  - *Mitigation*: 1 second is acceptable for chat start, but we can configure the interval or switch to long-polling later if needed.
- **[Risk] Daemon Network Outage** $\rightarrow$ If the Daemon loses internet connection, the task remains pending forever.
  - *Mitigation*: The background cleanup reaper already flags expired sessions and cleans up.
