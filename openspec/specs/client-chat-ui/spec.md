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
The system SHALL allow users to enter prompt text. When the user sends a prompt, the system SHALL send a POST request to `/sessions/{session_id}/messages` containing the prompt. The system SHALL disable the input field and the Send button during streaming, and update the chat area in real-time with response deltas received via the WebSocket connection.

#### Scenario: Stream response deltas
- **WHEN** the user submits a prompt
- **THEN** the system sends the prompt to the backend, disables input elements, and appends incoming WebSocket message text to the chat area.


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