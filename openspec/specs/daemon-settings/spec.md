# daemon-settings Specification

## Purpose

TBD - created by archiving change 'add-daemon-settings-ui'. Update Purpose after archive.

## Requirements

### Requirement: Local Settings Web UI
The Local Agent Daemon SHALL serve an HTML user interface on the root path (`GET /`) to allow local users to select the active CLI to run.

#### Scenario: Serve settings user interface
- **WHEN** the local user sends a GET request to `/`
- **THEN** the system SHALL return a HTTP 200 response with the settings HTML page containing a dropdown to select the active CLI and a Save button


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