## Why

To support advanced multimodal capabilities and human-in-the-loop validation, clients need the ability to upload image inputs, and local RAG developers need a way to inspect conversation context and provide manual responses to the client from the local daemon interface.

## What Changes

- **Base64 Message Attachments**: Message schemas on the Bridge are extended to support optional Base64-encoded image attachments in message requests.
- **Local Image Decoding on Host**: The Local Agent Daemon parses Base64 attachments, decodes them, saves them temporarily to disk, and passes the temporary file paths to the spawned CLI.
- **Daemon Conversation Viewer**: The local Daemon settings UI (Port 7456) will fetch and display conversation history for active sessions.
- **Human-in-the-Loop Manual Responses**: The Daemon settings UI provides an input field where developers can enter manual text responses. These can be sent continuously without locking the client, and images can be attached. Manual responses are blocked in AI mode to prevent interference.

## Capabilities

### Modified Capabilities

- `agent-bridge`: The message pipeline and execution schema are modified to support Base64 image attachments, local file decoding on the host, conversation history retrieval by the daemon, and manual response injection streaming. Additionally supports human/ready status synchronization and asynchronous operator message forwarding.
- `client-chat-ui`: The chat frontend is modified to allow users to select images, preview them, convert them to Base64, and send them as attachments with prompts. Supports unlocked inputs and direct rendering of operator-sent messages in human support mode.
- `daemon-settings`: The local daemon settings UI is modified to include a dashboard that displays active sessions, renders chat history, provides an input form for human manual responses (with attachment support), and buttons to toggle/restore AI mode.

## Impact

- Affected specs:
  - specs/agent-bridge/spec.md
  - specs/client-chat-ui/spec.md
  - specs/daemon-settings/spec.md
- Affected code:
  - Modified:
    - src/application/models/message.rs
    - src/infrastructure/db/sqlite.rs
    - src/infrastructure/db/message_repository_impl.rs
    - src/infrastructure/runtime/process_manager.rs
    - src/infrastructure/runtime/gemini_cli.rs
    - src/infrastructure/runtime/settings_ui.html
    - src/frontend/index.js
    - src/frontend/index.html
    - src/api/dto/message_dto.rs
