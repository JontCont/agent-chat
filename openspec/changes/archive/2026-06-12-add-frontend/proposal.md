## Why

Currently, the AI Agent prototype only exposes backend HTTP and WebSocket endpoints without a built-in user interface. Providing an integrated client chat UI served directly by the Axum server makes the project self-contained, easy to run, and convenient to deploy for client testing.

## What Changes

- Add a static client chat frontend served by the Axum backend.
- Update Axum configuration in `src/main.rs` to serve the static frontend directory.
- Update `Cargo.toml` to enable the `fs` feature in `tower-http` for static file serving.
- Implement the user interface with chat box, message history display, and controls.

## Capabilities

### New Capabilities

- `client-chat-ui`: A built-in user interface for real-time AI agent chat interaction.

### Modified Capabilities

(none)

## Impact

- Affected specs: `client-chat-ui`
- Affected code:
  - Modified:
    - `Cargo.toml`
    - `src/main.rs`
  - New:
    - `src/frontend/index.html`
    - `src/frontend/style.scss`
    - `src/frontend/style.css`
    - `src/frontend/index.js`
