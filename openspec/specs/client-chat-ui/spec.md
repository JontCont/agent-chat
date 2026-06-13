# client-chat-ui Specification

## Purpose

TBD - created by archiving change 'add-frontend'. Update Purpose after archive.

## Requirements

### Requirement: Embedded UI Layout
The system SHALL serve a web user interface at the root path (`/`). The interface MUST contain a chat message area, a text input field, a Send button, a Cancel button, and a connection status indicator.

#### Scenario: Serve user interface
- **WHEN** the client visits the root URL `/`
- **THEN** the system serves the chat interface containing the required UI elements.


<!-- @trace
source: add-frontend
updated: 2026-06-12
code:
  - src/frontend/index.js
  - Cargo.toml
  - src/frontend/index.html
  - src/frontend/style.scss
  - src/main.rs
  - src/frontend/style.css
-->

---
### Requirement: Automatic Session Initialization
The system SHALL automatically initialize a new chat session on page load. The system MUST send a POST request to `/sessions` to generate a new session UUID and retrieve its status.

#### Scenario: Initialize session on load
- **WHEN** the chat web page is loaded
- **THEN** the system automatically requests a new session and stores the returned session UUID.


<!-- @trace
source: add-frontend
updated: 2026-06-12
code:
  - src/frontend/index.js
  - Cargo.toml
  - src/frontend/index.html
  - src/frontend/style.scss
  - src/main.rs
  - src/frontend/style.css
-->

---
### Requirement: WebSocket Connection
The system SHALL establish a WebSocket connection to the endpoint `/ws/{session_id}` immediately after retrieving the session UUID.

#### Scenario: Connect WebSocket
- **WHEN** a session UUID is successfully initialized
- **THEN** the system establishes a WebSocket connection and updates the status indicator to show connected.


<!-- @trace
source: add-frontend
updated: 2026-06-12
code:
  - src/frontend/index.js
  - Cargo.toml
  - src/frontend/index.html
  - src/frontend/style.scss
  - src/main.rs
  - src/frontend/style.css
-->

---
### Requirement: Message Streaming and Input Locking
The system SHALL allow users to enter prompt text and attach local images. When the user sends a prompt, the system SHALL encode any attached images in Base64 and send a POST request to `/sessions/{session_id}/messages` containing the prompt and the image attachments. The system SHALL disable input fields and the Send button during streaming, and update the chat area in real-time with response deltas.

#### Scenario: Stream response deltas with image attachment
- **WHEN** the user attaches an image and submits a prompt
- **THEN** the system SHALL preview the image, convert it to Base64, send a POST request with the prompt text and Base64 attachment, disable inputs, and update the chat area with stream deltas


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
### Requirement: Execution Cancellation
The system SHALL allow users to cancel an active streaming run. When the user clicks the Cancel button, the system SHALL send a POST request to `/sessions/{session_id}/cancel`.

#### Scenario: Cancel active run
- **WHEN** the user clicks the Cancel button during active streaming
- **THEN** the system sends a cancellation request to the backend and re-enables the input field and Send button.

<!-- @trace
source: add-frontend
updated: 2026-06-12
code:
  - src/frontend/index.js
  - Cargo.toml
  - src/frontend/index.html
  - src/frontend/style.scss
  - src/main.rs
  - src/frontend/style.css
-->