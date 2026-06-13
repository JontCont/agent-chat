## Why

The current architecture requires the API server to make inbound HTTP calls to the Local Agent Daemon. When the API is deployed on a remote NAS and the Daemon runs on a Local PC behind a NAT router, the API cannot directly access the Daemon's port without manual firewall configuration, UPnP, or port forwarding. By reversing the communication direction to a pull-based model, the Daemon can make outbound HTTP requests to poll for tasks, allowing it to work seamlessly behind firewalls and NATs.

## What Changes

- Introduce a task-polling mechanism where the Local Agent Daemon polls the API server for execution tasks instead of receiving direct HTTP calls from the API server.
- The API server will store execution tasks (run prompt, cancel, mode settings) in a SQLite database table.
- Expose new internal HTTP endpoints on the API server for task polling and progress reporting.
- The Local Agent Daemon will run a background loop to poll the API server for pending tasks, execute them, and report stdout streams back to the API server via HTTP POST requests.
- The Daemon's local settings HTTP server will only bind to localhost (127.0.0.1) for security, as remote access is no longer required.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- agent-bridge: Change the communication protocol from push-based API-to-Daemon HTTP calls to pull-based Daemon-to-API task queue polling.
- daemon-settings: Modify the daemon settings UI and local API to use task polling for remote session synchronization and restrict the daemon settings server to localhost.

## Impact

- Affected specs:
  - openspec/specs/agent-bridge/spec.md
  - openspec/specs/daemon-settings/spec.md
- Affected code:
  - Modified:
    - src/main.rs
    - src/infrastructure/db/sqlite.rs
    - src/infrastructure/runtime/daemon_client.rs
    - src/infrastructure/runtime/gemini_cli.rs
    - src/api/routes/sessions.rs
