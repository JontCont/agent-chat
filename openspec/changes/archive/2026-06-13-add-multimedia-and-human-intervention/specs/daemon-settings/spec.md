## ADDED Requirements

### Requirement: Daemon Session History Viewer
The Local Agent Daemon settings UI SHALL support retrieving and displaying the conversation history of active sessions from the Bridge.

#### Scenario: Display conversation history
- **WHEN** the local developer selects an active session on the settings UI page
- **THEN** the system SHALL send a GET request to the Bridge to retrieve message history and render it in a chat log viewer on the settings page

---
### Requirement: Human-in-the-loop Manual Response
The Local Agent Daemon settings UI SHALL allow developers to type and send manual text responses to the client, which are streamed back to the Bridge as simulated CLI events.

#### Scenario: Send manual response
- **WHEN** the developer inputs a response and clicks the send button on the Daemon settings dashboard
- **THEN** the Daemon SHALL send a POST request to `/local/sessions/{session_id}/manual-response` containing the text and optional image attachments, and forward it to the Bridge which streams/notifies the client.

---
### Requirement: Operator Status Controls and Input Enforce
The Daemon settings UI SHALL only enable manual intervention when the session status is in Human Support mode. If in AI mode, the inputs SHALL be hidden or disabled. Toggling to human mode or back to AI mode SHALL be supported.

#### Scenario: Sync status and lock inputs
- **WHEN** the session is in AI mode
- **THEN** the Daemon settings UI SHALL display "AI Mode Active" and lock manual response forms. The operator can click "轉為人工客服" to force transition.
- **WHEN** the session is in Human mode
- **THEN** the Daemon settings UI SHALL display the input form, show attachment pickers for operator images, and provide a "切回AI模式" button.

