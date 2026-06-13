# daemon-settings Specification

## Purpose

TBD - created by archiving change 'add-daemon-settings-ui'. Update Purpose after archive.

## Requirements

### Requirement: Local Settings Web UI
The Local Agent Daemon SHALL serve an HTML user interface on the root path (`GET /`) bound to the local loopback interface (localhost) only, to allow local users to select the active CLI to run.

#### Scenario: Serve settings user interface
- **WHEN** the local user sends a GET request to `/` on the local loopback interface
- **THEN** the system SHALL return a HTTP 200 response with the settings HTML page containing a dropdown to select the active CLI and a Save button


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
### Requirement: Settings API Endpoints
The Local Agent Daemon SHALL expose REST API endpoints to retrieve and update the active CLI configuration.

#### Scenario: Retrieve active CLI configuration
- **WHEN** a GET request is sent to `/local/settings`
- **THEN** the system SHALL return a HTTP 200 response with a JSON body containing the active CLI name

##### Example: Get Settings Response
- **WHEN** the current active CLI is "agy"
- **THEN** the GET `/local/settings` response body SHALL be:
  ```json
  {
    "active_cli": "agy"
  }
  ```

#### Scenario: Update active CLI configuration
- **WHEN** a POST request is sent to `/local/settings` with a JSON payload containing a valid CLI name
- **THEN** the system SHALL update the configuration, persist it to disk, and return a HTTP 200 OK status

##### Example: Update Settings Payload
- **WHEN** updating the active CLI to "openai"
- **THEN** the POST `/local/settings` request body SHALL be:
  ```json
  {
    "active_cli": "openai"
  }
  ```


<!-- @trace
source: add-daemon-settings-ui
updated: 2026-06-13
code:
  - src/infrastructure/runtime/mod.rs
  - src/infrastructure/runtime/process_manager.rs
  - src/frontend/index.html
  - src/frontend/style.css
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
  - src/infrastructure/runtime/gemini_cli.rs
  - src/infrastructure/runtime/daemon_settings.rs
  - src/frontend/index.js
  - daemon_config.json
  - Cargo.toml
  - src/infrastructure/runtime/settings_ui.html
-->

---
### Requirement: Persistent Configuration File
The Local Agent Daemon SHALL load the active CLI configuration from a file named `daemon_config.json` in the project root directory during startup, and save the updated configuration to this file when modified.

#### Scenario: Configuration file exists on startup
- **GIVEN** a configuration file `daemon_config.json` with active CLI set to "openai"
- **WHEN** the Local Agent Daemon starts up
- **THEN** the system SHALL initialize with "openai" as the active CLI

#### Scenario: Configuration file does not exist
- **GIVEN** no configuration file `daemon_config.json` is present in the directory
- **WHEN** the Local Agent Daemon starts up or retrieves the configuration
- **THEN** the system SHALL default the active CLI to "agy"


<!-- @trace
source: add-daemon-settings-ui
updated: 2026-06-13
code:
  - src/infrastructure/runtime/mod.rs
  - src/infrastructure/runtime/process_manager.rs
  - src/frontend/index.html
  - src/frontend/style.css
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
  - src/infrastructure/runtime/gemini_cli.rs
  - src/infrastructure/runtime/daemon_settings.rs
  - src/frontend/index.js
  - daemon_config.json
  - Cargo.toml
  - src/infrastructure/runtime/settings_ui.html
-->

---
### Requirement: Platform CLI Mapping
The system SHALL map the configured active CLI to the appropriate executable command and resolve its executable path using platform environment variables.

#### Scenario: Execute configured CLI
- **WHEN** the daemon spawns the active CLI process
- **THEN** the system SHALL resolve the executable path using the corresponding environment variable, falling back to the default command name:
  | CLI Key | Environment Variable | Default Command Name |
  | --- | --- | --- |
  | `agy` | `AGY_CLI_PATH` | `agy` |
  | `openai` | `OPENAI_CLI_PATH` | `openai` |
  | `copilot` | `COPILOT_CLI_PATH` | `copilot` |
  | `claude` | `CLAUDE_CLI_PATH` | `claude` |

<!-- @trace
source: add-daemon-settings-ui
updated: 2026-06-13
code:
  - src/infrastructure/runtime/mod.rs
  - src/infrastructure/runtime/process_manager.rs
  - src/frontend/index.html
  - src/frontend/style.css
  - template/Gemini_Generated_Image_p0s1zep0s1zep0s1.png
  - src/infrastructure/runtime/gemini_cli.rs
  - src/infrastructure/runtime/daemon_settings.rs
  - src/frontend/index.js
  - daemon_config.json
  - Cargo.toml
  - src/infrastructure/runtime/settings_ui.html
-->

---
### Requirement: Daemon Session History Viewer
The Local Agent Daemon settings UI SHALL support retrieving and displaying the conversation history of active sessions from the Bridge.

#### Scenario: Display conversation history
- **WHEN** the local developer selects an active session on the settings UI page
- **THEN** the system SHALL send a GET request to the Bridge to retrieve message history and render it in a chat log viewer on the settings page


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
### Requirement: Human-in-the-loop Manual Response
The Local Agent Daemon settings UI SHALL allow developers to type and send manual text responses to the client, which are streamed back to the Bridge as simulated CLI events.

#### Scenario: Send manual response
- **WHEN** the developer inputs a response and clicks the send button on the Daemon settings dashboard
- **THEN** the Daemon SHALL send a POST request to `/local/sessions/{session_id}/manual-response` containing the text and optional image attachments, and forward it to the Bridge which streams/notifies the client.


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
### Requirement: Operator Status Controls and Input Enforce
The Daemon settings UI SHALL only enable manual intervention when the session status is in Human Support mode. If in AI mode, the inputs SHALL be hidden or disabled. Toggling to human mode or back to AI mode SHALL be supported.

#### Scenario: Sync status and lock inputs
- **WHEN** the session is in AI mode
- **THEN** the Daemon settings UI SHALL display "AI Mode Active" and lock manual response forms. The operator can click "轉為人工客服" to force transition.
- **WHEN** the session is in Human mode
- **THEN** the Daemon settings UI SHALL display the input form, show attachment pickers for operator images, and provide a "切回AI模式" button.

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