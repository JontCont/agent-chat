## Context

The local agent daemon currently executes `agy` CLI by default using the `AGY_CLI_PATH` environment variable. To support multi-model capability, users need to be able to run other CLIs (openai, copilot, claude) and configure which CLI is active locally through a secure, host-only settings UI served by the daemon.

## Goals / Non-Goals

**Goals:**
- Add local persistent configuration via `daemon_config.json`.
- Serve a settings UI HTML page on `GET /` of the Daemon (port 7456).
- Implement REST API endpoints `GET /local/settings` and `POST /local/settings` to query and update the active CLI.
- Update `process_manager.rs` to support `openai`, `copilot`, and `claude` CLI execution paths using platform environment variables (`OPENAI_CLI_PATH`, `COPILOT_CLI_PATH`, `CLAUDE_CLI_PATH`).
- Default the active CLI to `agy`.

**Non-Goals:**
- We do not integrate this configuration into the public Port 8080 chat UI.
- We do not configure or validate API keys/credentials for the CLIs in this UI.

## Decisions

### Decision: Direct HTML Embedding for Settings UI
- **Approach**: The settings HTML page will be embedded directly in the daemon binary using `include_str!` during compilation (reading from `src/infrastructure/runtime/settings_ui.html`). This ensures the daemon remains a self-contained executable that can be run from any directory without external asset dependencies.
- **Alternatives Considered**:
  - *Serve from disk directory*: Rejected because resolving relative static directories on the host is error-prone when run from different directories.
  - *React/Vue Frontend*: Rejected because a single HTML file with vanilla JS is extremely lightweight and sufficient for a single dropdown form.

### Decision: Local JSON Configuration File
- **Approach**: Persist the active CLI configuration in a `daemon_config.json` file in the current working directory of the running daemon.
- **Alternatives Considered**:
  - *SQLite Database*: Rejected because the daemon is stateless and doesn't connect to the SQLite DB pool initialized by the main Axum service.
  - *Environment Variables*: Rejected because environment variables cannot be written back persistently from the web UI.

## Implementation Contract

- **Behavior**:
  - Accessing `http://localhost:7456/` loads a settings web page.
  - Selecting a CLI and clicking "Save" updates the active CLI.
  - Spawning a run executes the active CLI with the prompt passed via `--print`.
- **Interface / Data Shape**:
  - `GET /` -> Serves `settings_ui.html`
  - `GET /local/settings` -> Returns JSON `{"active_cli": "agy"}`
  - `POST /local/settings` -> Receives JSON `{"active_cli": "openai"}` and updates config
  - JSON Config Schema (`daemon_config.json`):
    ```json
    {
      "active_cli": "agy"
    }
    ```
- **Failure Modes**:
  - Missing/corrupt config file: Defaults to `agy` silently.
  - Invalid CLI selection: Returns `400 Bad Request`.
  - Non-existent CLI execution: Triggers the existing mock generator/fallback.
- **Acceptance Criteria**:
  - `GET /` returns HTTP 200 and renders the settings dropdown.
  - `POST /local/settings` with invalid name returns HTTP 400.
  - `POST /local/settings` with valid name returns HTTP 200 and updates `daemon_config.json`.
  - Verify that spawning a process uses the environment variable path corresponding to the active CLI.
- **Scope Boundaries**:
  - *In-Scope*: settings web page, settings API, persistent JSON configuration, path mappings for `agy`, `openai`, `copilot`, `claude`.
  - *Out-of-Scope*: Setting API keys or endpoints in this UI.

## Risks / Trade-offs

- **[Risk] Config file path conflict** → *Mitigation*: Store `daemon_config.json` in the current directory and log the path clearly on startup.
- **[Risk] Execution failure when CLI not installed** → *Mitigation*: The daemon will fallback to the mock stream generator as it currently does for missing executables.
