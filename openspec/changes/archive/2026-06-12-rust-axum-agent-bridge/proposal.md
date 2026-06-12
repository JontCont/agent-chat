## Why

Establishing a robust, production-grade 1-to-1 AI Agent prototype that decouples the presentation layer (Axum API) from the runtime environment (Local Agent Daemon). This prevents direct process spawning from Axum handlers, manages process execution safely on NAS/Docker host environments, and enables real-time response streaming to frontends.

## What Changes

- Introduce a new `agent-bridge` capability with clear Presentation, Application, and Infrastructure boundaries.
- Set up a Dockerized Axum API that communicates with a Host-level Local Agent Daemon over TCP using `host.docker.internal`.
- Enable HTTP SSE (Server-Sent Events) streaming from the Daemon to the API, and WebSocket streaming from the API to the Frontend.
- Implement PID-based process tree tracking and cleanup in the Daemon to prevent orphan `gemini` CLI processes.
- Implement an async SQLite database access layer using `sqlx` in WAL mode to persist metadata and final responses.
- Implement an API-driven background Cleanup Reaper to terminate expired sessions and reclaim host resources.

## Capabilities

### New Capabilities

- `agent-bridge`: Core capability for executing Gemini CLI sessions, streaming response tokens, persistence of session metadata, and managing process lifecycles.

### Modified Capabilities

(none)

## Impact

- Affected specs:
  - `openspec/specs/agent-bridge/spec.md`
- Affected code:
  - New:
    - `src/main.rs`
    - `src/api/routes/health.rs`
    - `src/api/routes/sessions.rs`
    - `src/api/routes/websocket.rs`
    - `src/api/dto/session_dto.rs`
    - `src/api/dto/message_dto.rs`
    - `src/api/errors.rs`
    - `src/application/services/session_service.rs`
    - `src/application/services/message_service.rs`
    - `src/application/services/runtime_service.rs`
    - `src/application/services/cleanup_service.rs`
    - `src/application/models/session.rs`
    - `src/application/models/message.rs`
    - `src/application/models/events.rs`
    - `src/application/ports/session_repository.rs`
    - `src/application/ports/message_repository.rs`
    - `src/application/ports/runtime_gateway.rs`
    - `src/application/ports/ws_notifier.rs`
    - `src/infrastructure/db/sqlite.rs`
    - `src/infrastructure/db/session_repository_impl.rs`
    - `src/infrastructure/db/message_repository_impl.rs`
    - `src/infrastructure/runtime/daemon_client.rs`
    - `src/infrastructure/runtime/gemini_cli.rs`
    - `src/infrastructure/runtime/process_manager.rs`
    - `src/infrastructure/realtime/ws_registry.rs`
    - `src/infrastructure/config/env.rs`
    - `Cargo.toml`
    - `docker-compose.yml`
    - `Dockerfile`

