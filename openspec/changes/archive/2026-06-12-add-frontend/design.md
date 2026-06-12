## Context

Currently, the project contains the backend API service (Axum) and local daemon runtime (Gemini CLI client), but no user-facing UI. This design introduces an embedded client chat interface, written in Vanilla JS and SCSS, served statically by Axum, to provide a complete and self-contained chatbot application.

## Goals / Non-Goals

**Goals:**
- Enable Axum API to serve static assets from `src/frontend` natively at the root path (`/`).
- Build a responsive chat interface using HTML, Vanilla JS, and SCSS.
- Integrate the UI with session creation, WebSocket real-time event streaming, and cancellation endpoints.
- Maintain simple deployment with no build-time NPM dependencies for the web server.

**Non-Goals:**
- Supporting file uploads (no upload button or endpoint integration).
- Preserving or loading chat history across page refreshes (every refresh generates a new session).
- Building with a heavy frontend framework like React, Vue, or Next.js.

## Decisions

### Decision: Embedded Static File Serving via tower-http ServeDir
- **Choice**: Enable `fs` feature in `tower-http` and use `fallback_service(ServeDir::new("src/frontend"))` in the Axum router.
- **Rationale**: This allows the Axum server to natively serve the frontend assets, removing the need for a separate web server (e.g. Nginx). It simplifies deployment to a single binary/container and avoids CORS configuration.
- **Alternatives Considered**: 
  - *Separate Frontend Container*: Running Nginx alongside Axum via docker-compose. Rejected due to increased deployment complexity and CORS setup requirements.

### Decision: Vanilla JS with Pre-compiled SCSS
- **Choice**: Use Vanilla HTML5, Vanilla JavaScript, and SCSS compiled to static CSS.
- **Rationale**: Since the UI only has chat functionality, Vanilla JS keeps the build process zero-overhead (no npm or vite config needed). SCSS is used for easy styling maintenance, but compiles directly to a static `style.css` served by Axum.
- **Alternatives Considered**: 
  - *Vite + React / Svelte*: Rejected as it introduces package managers and build configurations which are unnecessary for a simple one-page chat application.

### Decision: Session Reset on Page Load (No History Recovery)
- **Choice**: Frontend requests a fresh session via `POST /sessions` on page load, and does not store the session ID in localStorage.
- **Rationale**: Fits the user's preference that every refresh or reopen represents a brand-new chat session, keeping session state management trivial.
- **Alternatives Considered**: 
  - *LocalStorage Persistence with Get History API*: Rejected because the user explicitly does not want to retain chat logs across loads.

## Implementation Contract

#### Observable Behavior
- **URL Root Access**: Accessing `http://localhost:8080/` serves `index.html`, rendering the chatbot interface.
- **Automated Session Creation**: The interface automatically retrieves a new session ID, connects to the WebSocket stream, and updates the status indicator to "Connected" on page load.
- **Interactive Chat and Locking**: The user inputs a prompt and hits Send. The prompt input and Send button are disabled, showing a busy state. The response streams back token-by-token.
- **Active Cancellation**: Clicking "Cancel" terminates the prompt execution, resets the inputs, and stops the stream immediately.

#### Interface & Data Shape
- **Static Assets**:
  - `src/frontend/index.html` - main document.
  - `src/frontend/style.scss` (with compiled `src/frontend/style.css`) - stylesheets.
  - `src/frontend/index.js` - JS logic.
- **Backend Routing**:
  - Add fallback static directory routing in `src/main.rs` to serve `src/frontend`.

#### Failure Modes
- **Daemon Unreachable**: The websocket client displays a descriptive connection/session error in the chat area when the backend fails to connect to the daemon.
- **WebSocket Disconnection**: The status indicator turns red and displays "Disconnected", disabling inputs and suggesting a page reload.

#### Acceptance Criteria
- Accessing `http://localhost:8080/` serves the chatbot UI successfully.
- Submitting a chat message successfully streams back simulated or real Gemini CLI response text deltas.
- Clicking the Cancel button during a stream terminates the run, deletes the PID file, and unlocks the input.

## Risks / Trade-offs

- **[Risk] SCSS Compilation Dependency**  
  *Mitigation*: We will write both the source `style.scss` and its pre-compiled `style.css` so that the application compiles and runs out of the box. Developers modifying styles can install the lightweight Sass compiler if needed.
