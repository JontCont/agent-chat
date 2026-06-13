# rust-axum-agent-bridge

A lightweight, self-hosted chat bridge that connects a browser-based chat UI to any local AI CLI tool (Gemini, Claude, OpenAI, Copilot, etc.) with real-time streaming.

## Architecture

```
Browser (WebSocket)
       │
       ▼
  Axum API Bridge  ──SSE──▶  Local Agent Daemon  ──spawn──▶  AI CLI Process
  (port 8080)                 (port 7456)                   (agy / claude / openai…)
       │
       ▼
    SQLite
```

The system consists of two processes:

| Process | Role |
|---------|------|
| **Axum API Bridge** | Serves the chat UI, manages sessions in SQLite, proxies prompts to the Daemon via SSE, and pushes streaming tokens to the browser over WebSocket |
| **Local Agent Daemon** | Runs on the host machine, spawns AI CLI child processes, streams output back, and manages process lifecycles |

## Features

- 🔄 **Real-time token streaming** — SSE from Daemon → Axum → WebSocket to browser
- 🖼️ **Image attachments** — Base64 image upload, decoded and passed to the CLI
- 🧑‍💼 **Human-in-the-loop** — Operator can take over a session and type manual responses via the Daemon settings UI
- 🔀 **Multi-CLI support** — Switch between `agy`, `claude`, `openai`, `copilot` at runtime without restart
- 🗃️ **SQLite persistence** — Session and message history stored locally
- 🧹 **Auto session cleanup** — Background reaper expires idle/timed-out sessions and kills orphan processes
- ❌ **Run cancellation** — Cancel an in-progress CLI execution mid-stream
- 🐳 **Docker support** — Single `docker compose up` deployment

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.78+
- At least one supported AI CLI installed and on your `PATH`:
  - [`agy`](https://github.com/your-org/agy) (default)
  - `claude`, `openai`, or `copilot`

### Run Locally (Development)

Open two terminals:

**Terminal 1 — Start the Daemon:**
```powershell
.\run_daemon.ps1
# Daemon listens on http://127.0.0.1:7456
```

**Terminal 2 — Start the API Bridge:**
```powershell
.\run_api.ps1
# API listens on http://localhost:8080
```

Open your browser at **http://localhost:8080**.

### Run with Docker

```bash
# Start everything (API Bridge only; Daemon must run on host)
docker compose up -d --build
```

> **Note:** The Daemon must run on the host machine (not in Docker) because it needs to spawn local CLI processes. The container connects to it via `host.docker.internal:7456`.

```bash
# View logs
docker compose logs -f

# Stop
docker compose down
```

## Configuration

All configuration is via environment variables (or a `.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `sqlite:///data/sqlite/agent.db` | SQLite database path |
| `DAEMON_URL` | `http://host.docker.internal:7456` | URL of the Local Agent Daemon |
| `PORT` | `8080` | Port for the Axum API Bridge |
| `DAEMON_PORT` | `7456` | Port the Daemon listens on |
| `BRIDGE_URL` | `http://127.0.0.1:8080` | URL the Daemon uses to call back the Bridge (set to your public domain when deploying remotely) |

### CLI Path Overrides

The Daemon resolves CLI executables via environment variables:

| CLI Key | Env Variable | Default |
|---------|-------------|---------|
| `agy` | `AGY_CLI_PATH` | `agy` |
| `openai` | `OPENAI_CLI_PATH` | `openai` |
| `copilot` | `COPILOT_CLI_PATH` | `copilot` |
| `claude` | `CLAUDE_CLI_PATH` | `claude` |

### Daemon Configuration

The active CLI is persisted to `daemon_config.json` in the project root:

```json
{
  "active_cli": "agy"
}
```

You can also change it at runtime via the Daemon settings UI at **http://localhost:7456**.

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `POST` | `/sessions` | Create a new chat session |
| `POST` | `/sessions/:id/messages` | Send a prompt (with optional image attachments) |
| `POST` | `/sessions/:id/cancel` | Cancel an in-progress run |
| `GET` | `/ws/:id` | WebSocket — subscribe to session token stream |

### Message Payload

```json
{
  "content": "What is in this image?",
  "attachments": [
    {
      "mime_type": "image/png",
      "data": "<base64-encoded-image>"
    }
  ]
}
```

## Project Structure

```
src/
├── main.rs                    # Entry point (API Bridge or Daemon mode)
├── api/                       # HTTP routes and DTOs
│   ├── routes/
│   │   ├── sessions.rs        # Session and message endpoints
│   │   └── websocket.rs       # WebSocket endpoint
│   └── dto/
├── application/               # Business logic (services, ports, models)
│   ├── services/
│   │   ├── session_service.rs
│   │   ├── runtime_service.rs
│   │   └── cleanup_service.rs
│   ├── models/
│   └── ports/
└── infrastructure/            # Adapters (DB, Daemon client, WebSocket registry)
    ├── config/env.rs          # Environment config
    ├── db/                    # SQLite repositories
    ├── realtime/              # WebSocket registry
    └── runtime/               # Daemon client, process manager, settings UI

src/frontend/                  # Browser chat UI (HTML/CSS/JS)
```

## Tech Stack

- **[Axum](https://github.com/tokio-rs/axum)** — Async HTTP + WebSocket server
- **[Tokio](https://tokio.rs/)** — Async runtime
- **[SQLx](https://github.com/launchbadder/sqlx)** — SQLite with async support
- **[reqwest](https://github.com/seanmonstar/reqwest)** — SSE streaming HTTP client
- **[tower-http](https://github.com/tower-rs/tower-http)** — CORS, static file serving

## License

MIT
