## Context

In a typical NAS Docker setup, running resource-heavy and credential-based command-line utilities (like the Gemini CLI) directly inside a Docker container is problematic. Containers often lack access to local filesystems, host tools, and authentication contexts, and spawning child processes directly inside API route handlers creates coupling, risk of resource leakage, and lack of horizontal scalability. 

To resolve this, we decouple the system into:
1. **Axum API (Docker Container)**: Handles client HTTP requests, WebSocket real-time connections, metadata persistence, and session orchestration.
2. **Local Agent Daemon (NAS Host)**: A lightweight daemon running natively on the host that manages the lifecycle of the Gemini CLI child processes, stream piping, and PID tracking.

## Goals / Non-Goals

**Goals:**
- Decouple API handlers from CLI process lifecycles.
- Enable token-by-token streaming from Host to API via SSE, and API to Client via WebSocket.
- Prevent orphan `gemini` CLI processes on the host via PID group termination.
- Persist session metadata and final complete answers in SQLite using WAL mode.
- Clean up inactive sessions and terminate associated host processes automatically using an API-driven Reaper.

**Non-Goals:**
- Implementing multi-tenant authentication, quotas, or user registration.
- Implementing an interactive bash-like persistent shell session; all CLI prompt executions in this phase are one-shot commands.
- Bundling Gemini CLI or google-genai SDK within the Axum Docker container.

## Decisions

### Decision: API-to-Daemon Host TCP Network
- **Choice**: Use TCP HTTP over `host.docker.internal` mapping to the host's port `7456`.
- **Rationale**: Keeps implementation standard and allows using simple HTTP clients. The `extra_hosts` option in `docker-compose.yml` provides high portability across different OS host environments.
- **Alternatives Considered**: 
  - *Unix Domain Sockets*: Safer but requires complex Docker volume mounting of the socket file and non-standard Rust HTTP client setup.
  - *Single Unified Container*: Packaging the Gemini CLI and SDKs inside the API container, which was rejected due to host dependency issues and credential exposure.

### Decision: Server-Sent Events (SSE) for Stream Transport
- **Choice**: Use HTTP `text/event-stream` for Daemon-to-API communication.
- **Rationale**: SSE fits naturally into RESTful unidirectional streaming. Axum and standard Rust HTTP libraries have built-in support for streaming responses, keeping state management simple.
- **Alternatives Considered**:
  - *WebSockets*: Bi-directional and complex, requiring connection registries and heartbeat management on both ends.
  - *Webhook Callback*: Rejected due to high overhead of repeated HTTP POST requests for every token delta.

### Decision: PID-Based Process Group Termination
- **Choice**: Daemon writes child process PIDs to `/tmp/agent-pids/{session_id}.pid` and kills the process group during cancel/cleanup (using OS-specific command trees).
- **Rationale**: Ensures that even if the Daemon crashes, the background cleanup process can read the directory and kill orphaned CLI processes, avoiding memory/CPU leaks.
- **Alternatives Considered**:
  - *In-memory Child Handles*: Simple but vulnerable; when Daemon restarts, all references are lost, leaving child processes running forever.

### Decision: sqlx Async SQLite Database Driver
- **Choice**: Use `sqlx` in WAL (Write-Ahead Logging) mode.
- **Rationale**: Provides native async support that integrates perfectly with Axum. WAL mode allows concurrent reads during writes.
- **Alternatives Considered**:
  - *rusqlite + r2d2*: Synchronous and blocking, requiring `tokio::task::spawn_blocking` wrappers around all database operations.
  - *Diesel*: Heavy ORM with steep setup cost, overkill for our simple two-table schema.

### Decision: API-Led Reaper Cleanup
- **Choice**: The background cleanup thread runs in the API container, queries SQLite for expired sessions, and sends `DELETE` commands to the Daemon to terminate active runs.
- **Rationale**: Keeps the business logic (expiration timers, database status updates) centralized in the API Application Layer.

## Implementation Contract

### Observable Behavior
- **Session Lifecycle**: Clients can create a session (`POST /sessions`), view its status (`GET /sessions/{id}`), connect to real-time events (`GET /ws/{id}`), send messages (`POST /sessions/{id}/messages`), and cancel active runs (`POST /sessions/{id}/cancel`).
- **Streaming response**: Messages yield real-time chunks on the WebSocket client.

### Interface & Data Shape
- **HTTP Endpoints (API)**:
  - `POST /sessions` -> returns `201 Created` with `{ "id": "uuid", "status": "ready" }`
  - `POST /sessions/{id}/messages` -> Body `{ "content": "prompt" }`. Returns `202 Accepted` (or streams via WebSocket)
  - `POST /sessions/{id}/cancel` -> returns `200 OK`
  - `DELETE /sessions/{id}` -> returns `200 OK`
  - `GET /ws/{id}` -> Upgrades to WebSocket.
- **HTTP Endpoints (Daemon)**:
  - `POST /local/sessions/{id}/messages` -> Body `{ "content": "prompt" }`. Returns `text/event-stream` with SSE format (`event: delta`, `event: done`, `event: error`).
  - `POST /local/sessions/{id}/cancel` -> returns `200 OK`
  - `DELETE /local/sessions/{id}` -> returns `200 OK` (terminates active process and deletes PID file)
- **SQLite Schema**:
  - `sessions` table (fields: `id`, `status`, `created_at`, `last_seen_at`, `expires_at`, `disconnected_at`, `runtime_id`)
  - `messages` table (fields: `id`, `session_id`, `role`, `content`, `created_at`, `is_final`)

### Failure Modes
- **Daemon Unreachable**: API fails to connect to `host.docker.internal:7456`. API transitions the session status to `faulted` in SQLite, sends a `session.error` frame over the WebSocket, and returns `503 Service Unavailable` to the HTTP caller.
- **CLI Exits with Error**: Daemon streams `event: error` containing stderr details, terminates execution, and deletes the PID file.

### Acceptance Criteria
- Running `cargo test` runs database migration scripts and verifies that repository queries execute successfully.
- Triggering `POST /sessions/{id}/cancel` on a running long prompt successfully terminates the process tree on the host and deletes `/tmp/agent-pids/{id}.pid`.

### Scope Boundaries
- **In Scope**:
  - Full API endpoints (`/sessions`, `/sessions/{id}/messages`, `/sessions/{id}/cancel`, `/ws/{id}`).
  - Full Daemon implementation endpoints on port `7456` with process execution and PID management.
  - SQLite persistence layer with `sqlx`.
  - Periodic background cleanup logic in the API.
- **Out of Scope**:
  - User authorization / authentication layer.
  - Multi-host distribution of the Daemon.
  - Persistent interactive CLI session.

## Risks / Trade-offs

- **[Risk] Docker Host Networking Complexity**  
  *Mitigation*: Ensure `host-gateway` is correctly registered in the Docker service configuration.
- **[Risk] Cross-Platform PID Control**  
  *Mitigation*: Use Rust conditional compilation (`#[cfg(windows)]` and `#[cfg(unix)]`) in the Daemon's process manager to target `taskkill` on Windows and `nix::sys::signal` on Linux.

