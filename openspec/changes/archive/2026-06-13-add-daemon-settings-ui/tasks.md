## 1. Configuration Persistence and Settings API

- [x] 1.1 Implement loading and saving of the active CLI selection from/to a daemon_config.json file in src/infrastructure/runtime/daemon_settings.rs. Deliver the "Persistent Configuration File" requirement and default to "agy" if the file is missing. Verify by running cargo tests for settings serialization and deserialization. Reference design decision "Decision: Local JSON Configuration File".
- [x] 1.2 Add router routes and handlers for GET /local/settings and POST /local/settings in src/infrastructure/runtime/gemini_cli.rs. Deliver the "Settings API Endpoints" requirement allowing retrieval and updates of the active CLI config. Verify by calling the endpoints via HTTP requests and asserting correct JSON response payloads/status codes.

## 2. Web Settings UI

- [x] 2.1 Create an HTML settings UI page in src/infrastructure/runtime/settings_ui.html. Deliver the "Local Settings Web UI" requirement showing the select dropdown for CLIs (agy, openai, copilot, claude) and Save button. Verify by content review.
- [x] 2.2 Mount a GET / route in the Daemon Router (src/infrastructure/runtime/gemini_cli.rs) to serve the HTML page using include_str!. Reference design decision "Decision: Direct HTML Embedding for Settings UI". Verify by launching the daemon locally and loading http://127.0.0.1:7456 in a browser.

## 3. CLI Platform Mapping and Execution

- [x] 3.1 Modify process_manager.rs to support executing different active CLI types. Deliver the "Platform CLI Mapping" requirement mapping the selection to AGY_CLI_PATH, OPENAI_CLI_PATH, COPILOT_CLI_PATH, and CLAUDE_CLI_PATH environment variables. Verify by unit tests or manual CLI execution path assertions.
- [x] 3.2 Update process spawning in process_manager.rs and gemini_cli.rs to read the active CLI selection dynamically from the settings file during message runs. Deliver the "Real-time Message Streaming" requirement to spawn the configured CLI instead of hardcoding Gemini CLI. Verify by executing a message run and verifying in logs that the daemon attempts to run the configured command.
