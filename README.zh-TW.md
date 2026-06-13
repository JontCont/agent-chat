# rust-axum-agent-bridge

輕量、可自架的聊天橋接器，將瀏覽器聊天介面連接到任何本機 AI CLI 工具（Gemini、Claude、OpenAI、Copilot 等），支援即時串流輸出。

## 架構

```
瀏覽器（WebSocket）
       │
       ▼
  Axum API 橋接器  ◀──輪詢/進度回報──  本機 Agent Daemon  ──spawn──▶  AI CLI 程序
  （port 8080）                         （port 7456, 本機）           （agy / claude / openai…）
       │
       ▼
 SQLite (任務佇列資料表)
```

系統由兩個程序組成：

| 程序 | 職責 |
|------|------|
| **Axum API 橋接器** | 提供聊天 UI、以 SQLite 管理 Session、在 SQLite 中建立任務佇列，並以 WebSocket 將串流 Token 推送至瀏覽器 |
| **本機 Agent Daemon** | 執行在宿主機上，向 API 橋接器輪詢任務、啟動 AI CLI 子程序，並透過 HTTP 將執行進度串流回傳 |

## 功能特色

- 🔄 **即時 Token 串流** — Daemon → Axum（HTTP 進度回報）→ 瀏覽器（WebSocket）端對端串流
- 🖼️ **圖片附件** — 支援 Base64 圖片上傳，自動解碼後傳入 CLI
- 🧑‍💼 **人工介入（Human-in-the-loop）** — 客服人員可透過 Daemon 設定 UI 接管 Session，手動輸入回覆
- 🔀 **多 CLI 支援** — 執行期間切換 `agy`、`claude`、`openai`、`copilot`，無需重啟
- 🗃️ **SQLite 持久化** — Session 與訊息歷史本機儲存
- 🧹 **自動 Session 清理** — 背景任務定期清除閒置或逾時的 Session 並終止孤立程序
- ❌ **取消執行** — 可中途取消進行中的 CLI 執行
- 🐳 **Docker 支援** — 一行 `docker compose up` 即可部屬

## 快速開始

### 前置需求

- [Rust](https://rustup.rs/) 1.78+
- 至少安裝一個支援的 AI CLI 並加入 `PATH`：
  - [`agy`](https://github.com/your-org/agy)（預設）
  - `claude`、`openai` 或 `copilot`

### 本機開發執行

開啟兩個終端機：

**終端機 1 — 啟動 Daemon：**
```powershell
.\run_daemon.ps1
# Daemon 會向 API 橋接器輪詢任務，並在 http://127.0.0.1:7456 提供本地設定 UI
```

**終端機 2 — 啟動 API 橋接器：**
```powershell
.\run_api.ps1
# API 監聽於 http://localhost:8080
```

用瀏覽器開啟 **http://localhost:8080**。

### 使用 Docker 部屬

```bash
# 啟動（僅 API 橋接器；Daemon 需在宿主機上執行）
docker compose up -d --build
```

> **注意：** Daemon 必須執行在宿主機（非 Docker 容器內），因為它需要在本機啟動 CLI 程序。Daemon 會使用 `BRIDGE_URL` 環境變數向 API 橋接器進行 Outbound 輪詢。

```bash
# 查看 log
docker compose logs -f

# 停止
docker compose down
```

## 設定

所有設定均透過環境變數（或 `.env` 檔案）提供：

| 變數 | 預設值 | 說明 |
|------|--------|------|
| `DATABASE_URL` | `sqlite:///data/sqlite/agent.db` | SQLite 資料庫路徑 |
| `PORT` | `8080` | Axum API 橋接器監聽 Port |
| `DAEMON_PORT` | `7456` | Daemon 本地設定服務監聽 Port |
| `BRIDGE_URL` | `http://127.0.0.1:8080` | Daemon 連接與輪詢 API 橋接器（Bridge）的 URL（遠端部屬時設為對外公網 domain） |

### CLI 執行路徑覆寫

Daemon 透過環境變數解析 CLI 執行檔路徑：

| CLI 鍵值 | 環境變數 | 預設指令 |
|---------|----------|---------|
| `agy` | `AGY_CLI_PATH` | `agy` |
| `openai` | `OPENAI_CLI_PATH` | `openai` |
| `copilot` | `COPILOT_CLI_PATH` | `copilot` |
| `claude` | `CLAUDE_CLI_PATH` | `claude` |

### Daemon 設定檔

目前啟用的 CLI 會持久化到專案根目錄的 `daemon_config.json`：

```json
{
  "active_cli": "agy"
}
```

也可在執行期間透過 Daemon 設定 UI（**http://localhost:7456**）即時切換。

## API 文件

| 方法 | 端點 | 說明 |
|------|------|------|
| `GET` | `/health` | 健康檢查 |
| `POST` | `/sessions` | 建立新的聊天 Session |
| `POST` | `/sessions/:id/messages` | 送出提示（可附帶圖片） |
| `POST` | `/sessions/:id/cancel` | 取消進行中的執行 |
| `GET` | `/ws/:id` | WebSocket — 訂閱 Session 的 Token 串流 |

### 訊息 Payload 格式

```json
{
  "content": "這張圖片裡有什麼？",
  "attachments": [
    {
      "mime_type": "image/png",
      "data": "<base64-encoded-image>"
    }
  ]
}
```

## 專案結構

```
src/
├── main.rs                    # 進入點（API 橋接器或 Daemon 模式）
├── api/                       # HTTP 路由與 DTO
│   ├── routes/
│   │   ├── sessions.rs        # Session 與訊息端點
│   │   └── websocket.rs       # WebSocket 端點
│   └── dto/
├── application/               # 商業邏輯（服務、Port、模型）
│   ├── services/
│   │   ├── session_service.rs
│   │   ├── runtime_service.rs
│   │   └── cleanup_service.rs
│   ├── models/
│   └── ports/
└── infrastructure/            # 介面卡（DB、Daemon 客戶端、WebSocket 登錄表）
    ├── config/env.rs          # 環境設定
    ├── db/                    # SQLite Repository
    ├── realtime/              # WebSocket 登錄表
    └── runtime/               # Daemon 客戶端、程序管理、設定 UI

src/frontend/                  # 瀏覽器聊天 UI（HTML/CSS/JS）
```

## 技術棧

- **[Axum](https://github.com/tokio-rs/axum)** — 非同步 HTTP + WebSocket 伺服器
- **[Tokio](https://tokio.rs/)** — 非同步執行環境
- **[SQLx](https://github.com/launchbadder/sqlx)** — 非同步 SQLite 支援
- **[reqwest](https://github.com/seanmonstar/reqwest)** — SSE 串流 HTTP 客戶端
- **[tower-http](https://github.com/tower-rs/tower-http)** — CORS、靜態檔案服務

## 授權

MIT
