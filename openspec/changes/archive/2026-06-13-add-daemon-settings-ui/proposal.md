## Why

The local daemon currently only supports execution of the Gemini/agy CLI, and its configuration cannot be modified at runtime. To support multi-model capability, users need to be able to run other CLIs (OpenAI, Copilot, Claude) and configure which CLI is active locally through a secure, host-only interface.

## What Changes

- **Multiple CLI Support**: The Local Agent Daemon will support executing openai, copilot, and claude CLIs in addition to the default agy CLI.
- **Environment Variables**: Each CLI path will be configurable via specific environment variables: AGY_CLI_PATH, OPENAI_CLI_PATH, COPILOT_CLI_PATH, and CLAUDE_CLI_PATH.
- **Local Settings Web UI**: The Local Agent Daemon (port 7456) will serve a simple web settings page at GET / allowing local developers to select the active CLI.
- **Persistent Configuration**: Settings will be saved persistently to a local JSON file daemon_config.json on the host machine.
- **Stateless Message Execution**: The active CLI will be resolved dynamically by the daemon when executing a session message.

## Capabilities

### New Capabilities

- daemon-settings: The Local Agent Daemon serves a settings HTML UI at the root path (/) to configure the active CLI, and persists this setting to daemon_config.json on the host.

### Modified Capabilities

- agent-bridge: The session execution requirements are modified so the daemon executes the configured CLI (defaulting to agy) rather than hardcoding a single CLI.

## Impact

- Affected specs:
  - specs/daemon-settings/spec.md
  - specs/agent-bridge/spec.md
- Affected code:
  - New:
    - src/infrastructure/runtime/daemon_settings.rs
    - src/infrastructure/runtime/settings_ui.html
  - Modified:
    - src/infrastructure/runtime/process_manager.rs
    - src/infrastructure/runtime/gemini_cli.rs
    - src/main.rs
