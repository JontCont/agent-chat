## 1. Database and Model Setup

- [x] 1.1 Modify sqlite.rs to run an ALTER TABLE command on startup adding the attachments column to the messages table, and update the Message struct and message_repository_impl.rs to persist message media. Reference design decision "Decision: Base64 Serialized Media Attachments in Messages". Verify by verifying compilation and database schema updates.

## 2. Image Decoding on Daemon Host

- [x] 2.1 Implement Base64 parsing and temporary image file writing in process_manager.rs. Deliver the "Real-time Message Streaming" requirement. Reference design decision "Decision: Local Host Image Temp File Writing". Verify by running unit tests that assert files are written to disk and cleaned up after execution.

## 3. Daemon Human-in-the-loop and History API

- [x] 3.1 Implement GET and POST endpoints for retrieving session history and submitting human operator text overrides in gemini_cli.rs. Deliver the "Daemon Session History Viewer" and "Human-in-the-loop Manual Response" requirements. Reference design decision "Decision: Manual Response Interceptor via Tokio Channel". Verify by calling the endpoints and checking response body and stream output.

## 4. Frontend Client and Daemon UI Integration

- [x] 4.1 Update frontend index.html and index.js to allow users to select images, generate Base64 strings, and send them inside message requests. Deliver the "Message Streaming and Input Locking" requirement. Verify by selecting an image and asserting the POST payload matches the attachments schema.
- [x] 4.2 Extend settings_ui.html to render active session chat logs and provide a manual override input interface. Verify by loading the settings page and performing a manual override check.

## 5. Enforce AI-Mode Handoff Restrictions and Continuous Operator Messaging

- [x] 5.1 Restrict Daemon `post_manual_response` override to only process messages if session `is_human` is true, otherwise return `400 Bad Request`.
- [x] 5.2 Implement state synchronization routes on Daemon (`POST /local/sessions/:id/human` and `POST /local/sessions/:id/ready`) and Bridge (notifying Daemon via client calls on status transition).
- [x] 5.3 Implement Bridge endpoints: `POST /sessions/:id/ready` to restore to AI mode, and `POST /sessions/:id/operator-response` to save and notify operator messages to the client.
- [x] 5.4 Update client frontend `index.js` to prevent input locking during human mode, and listen for `operator.message` WebSocket notifications.
- [x] 5.5 Update Daemon `settings_ui.html` to hide the override panel during AI mode, show a "Transfer to Human" / "Restore to AI" toggle button, allow operator image uploading, and render attachments in chat history.

