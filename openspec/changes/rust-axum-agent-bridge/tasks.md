## 1. Setup and Infrastructure

- [x] 1.1 Implement Cargo.toml dependencies and docker-compose.yml configuration with extra_hosts to enable Decision: API-to-Daemon Host TCP Network, verified by running `docker-compose config` successfully.
- [x] 1.2 Implement the sqlite initialization code in `src/infrastructure/db/sqlite.rs` using sqlx in WAL mode to support the Decision: sqlx Async SQLite Database Driver and run migrations to create the sessions and messages tables, verified by running database migrations successfully.

## 2. Session Creation

- [x] 2.1 Implement the endpoint for Session Creation at POST /sessions in `src/api/routes/sessions.rs`, generating a UUID and saving it to SQLite in WAL mode with status starting and transitioning to ready to define the Session Lifecycle under Observable Behavior, verified by a curl test returning 201 Created and JSON containing the session ID.

## 3. Real-time Message Streaming

- [x] 3.1 Implement the Local Agent Daemon's prompt execution in `src/infrastructure/runtime/gemini_cli.rs` and `src/infrastructure/runtime/process_manager.rs`, spawning the gemini CLI as a child process and streaming stdout/stderr back as HTTP chunked Server-Sent Events to satisfy the Interface & Data Shape of the streaming daemon endpoint, and support Decision: Server-Sent Events (SSE) for Stream Transport and the Real-time Message Streaming requirement, verified by querying the daemon directly with a mock prompt and receiving structured SSE event frames.
- [x] 3.2 Implement WebSocket upgrading at GET /ws/{id} in `src/api/routes/websocket.rs` and `src/infrastructure/realtime/ws_registry.rs`, upgrading HTTP connections to WebSocket and bridging the SSE stream events from the Daemon to the WebSocket client within the defined Scope Boundaries, verified by connecting a websocket client (e.g., websocat) and receiving JSON frames containing stream deltas.

## 4. Run Cancellation

- [x] 4.1 Implement Decision: PID-Based Process Group Termination in the Local Agent Daemon, recording the child process PID to `/tmp/agent-pids/{session_id}.pid` and implementing a cancel endpoint to kill the process group for the Run Cancellation requirement, satisfying the cancel command Acceptance Criteria, verified by triggering a cancel request during a mock execution and asserting that the child process is terminated and the PID file is deleted.
- [x] 4.2 Expose the cancellation API endpoint at POST /sessions/{id}/cancel in the Axum API to forward the request to the Daemon, verified by calling the endpoint and observing a success response and child process termination.

## 5. SQLite Persistence and Background Cleanup

- [x] 5.1 Implement the database repository for SQLite Persistence in `src/infrastructure/db/session_repository_impl.rs` and `src/infrastructure/db/message_repository_impl.rs` to save final response messages and session updates, verified by running integration tests that assert data is written correctly.
- [x] 5.2 Implement the API background reaper task in `src/application/services/cleanup_service.rs` to satisfy the Decision: API-Led Reaper Cleanup, handling unreachable daemon Failure Modes and the Background Cleanup requirement, periodically calling the Daemon DELETE endpoint for expired sessions and updating SQLite status to expired, verified by a unit test asserting expired database sessions are deleted/marked and cancel commands sent.


