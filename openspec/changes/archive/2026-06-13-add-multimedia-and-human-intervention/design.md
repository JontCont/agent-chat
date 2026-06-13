## Context

To support advanced multimodal capabilities and human-in-the-loop validation, clients need the ability to upload image inputs, and local RAG developers need a way to inspect conversation context and provide manual responses to the client from the local daemon interface.

## Goals / Non-Goals

**Goals:**
- Add Base64 image attachment serialization and persistence to conversation messages.
- Enable Local Agent Daemon to decode Base64 attachments to temporary files and pass paths to CLI executables.
- Expose endpoints on the Daemon for conversation history retrieval and manual response stream injection.
- Serve a human intervention form/dashboard in `settings_ui.html` on the Daemon (Port 7456).

**Non-Goals:**
- We do not support uploading non-image assets (PDFs, ZIPs, etc.).
- We do not support editing or deleting historical messages from the Daemon UI.

## Decisions

### Decision: Base64 Serialized Media Attachments in Messages
- **Approach**: We will extend the `messages` table in `sqlite.rs` with an `attachments` column of type `TEXT` that holds a serialized JSON array of media attachments: `[{"mime_type": "image/png", "data": "base64_string..."}]`. The frontend converts image files to Base64 in JavaScript, sends them inside the `PromptRequest` payload, and the Bridge stores and forwards them to the Daemon.
- **Alternatives Considered**:
  - *Separate file upload endpoints and disk storage*: Rejected because of the additional complexity of file upload synchronization, auth checks, and cleanup management across the client-bridge-daemon layers.

### Decision: Local Host Image Temp File Writing
- **Approach**: In `process_manager.rs`, the Daemon parses incoming Base64 image attachments, decodes them to byte vectors, and writes them to unique temporary files in the host system's temporary directory. The temporary file paths are then passed as CLI arguments, and deleted from disk once the CLI execution finishes.
- **Alternatives Considered**:
  - *Passing Base64 directly via stdin or CLI arg*: Rejected because many image-capable CLI tools require physical file paths.

### Decision: Manual Response Interceptor via Tokio Channel
- **Approach**: The Daemon will maintain a registry of active session output senders. A new endpoint `POST /local/sessions/:id/manual-response` will accept `{ "content": "..." }` and write this content directly to the corresponding active session's stream sender, simulating a CLI output stream.
- **Alternatives Considered**:
  - *Bridge websocket injection*: Rejected because it bypasses the Daemon, which is supposed to act as the single source of truth for execution runs.

## Implementation Contract

- **Behavior**:
  - Chat UI shows an attachment picker and a preview.
  - Image attachments are sent as Base64 JSON and saved to SQLite.
  - Daemon UI retrieves conversation history and allows manual response streaming.
- **Interface / Data Shape**:
  - Attachment structure in JSON:
    ```json
    "attachments": [
      {
        "mime_type": "image/png",
        "data": "iVBORw0KGgoAAAANS..."
      }
    ]
    ```
  - Manual Response Endpoint: `POST /local/sessions/:id/manual-response` taking `{ "content": "...", "attachments": [...] }`
  - Bridge Operator Response Endpoint: `POST /sessions/:id/operator-response` taking `{ "content": "...", "attachments": [...] }`
  - Synchronization Endpoints: `POST /local/sessions/:id/human` and `POST /local/sessions/:id/ready` on the Daemon.
- **Failure Modes**:
  - Missing or corrupt Base64 data: Skips image processing and logs warning.
  - Posting manual response to completed session: Returns `409 Conflict` or `404 Not Found`.
  - Operator attempting override in AI mode: Returns `400 Bad Request`.
- **Acceptance Criteria**:
  - Verify that image selection generates a Base64 string and posts it inside the JSON request body.
  - Verify that the Daemon decodes the Base64 image to a temporary file path and executes the CLI with the path.
  - Verify that submitting a manual response from `settings_ui.html` sends the text directly to the Bridge, persisting it and notifying client.
  - Verify that operator form is locked/hidden while in AI mode.
- **Scope Boundaries**:
  - *In-Scope*: Base64 image encoding/decoding, SQLite attachments column, settings UI dashboard, manual response HTTP endpoint and stream injection, operator image upload and preview, AI restore capabilities.
  - *Out-of-Scope*: Real-time audio/video streaming, multi-file uploads.

## Risks / Trade-offs

- **[Risk] High memory usage for huge images** → *Mitigation*: Restrict frontend image selection to under 5MB and resize/compress if needed.
- **[Risk] Temporary file leakage on process panic** → *Mitigation*: Wrap file writing and cleanup in a scoped RAII struct or robust `defer`-like cleanup blocks.
