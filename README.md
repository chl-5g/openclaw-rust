# OpenAgentic

**Open-source AI phone agent** — Let AI operate your Android phone

[English](#architecture) | [中文](#中文说明)

An AI Agent platform built on Android Accessibility Services. Users describe tasks in natural language; the AI understands the screen and controls the phone to complete operations. Supports 100+ LLM APIs, local-first data, open-source and auditable.

## Architecture

```
Android App (Kotlin, Jetpack Compose)
    ↕  HTTP / SSE / WebSocket
Rust Gateway (axum + LiteLLM, port 18789)
    ↕
100+ LLM APIs (OpenAI / Anthropic / Gemini / DeepSeek / Qwen / Ollama ...)
```

## Features

- **100+ LLM support** — Unified LiteLLM gateway: OpenAI, Anthropic, Gemini, DeepSeek, Qwen, Ollama, and more
- **Multi-agent system** — Task decomposition across specialized agents (Researcher, Coder, Writer)
- **3-layer memory** — Working → short-term (compressed) → long-term (vector store)
- **Phone control** — Read screen via AccessibilityService, simulate taps/swipes/typing (planned)
- **Voice interaction** — STT speech recognition + TTS synthesis
- **Security** — JWT + Argon2 auth, input filtering, output validation, audit logging, rate limiting
- **Multi-channel** — 15+ messaging integrations (Telegram, Discord, DingTalk, WeCom, Feishu)
- **Tool ecosystem** — Browser automation, cron jobs, webhooks, MCP integration

## Quick Start

```bash
# Clone and build
git clone https://github.com/openagentic-ai/open-agentic.git
cd open-agentic
cargo build --release

# Start the gateway
./target/release/open-agentic gateway

# Health check
curl http://localhost:18789/health
```

### Setup Authentication

```bash
# Generate a password hash
./target/release/open-agentic hash-password YOUR_PASSWORD

# Configure ~/.openclaw-rust/config.json:
# {
#   "security": {
#     "jwt_secret": "your-random-secret-key",
#     "admin_username": "admin",
#     "admin_password_hash": "$argon2id$v=19$..."
#   }
# }

# Login
curl -X POST http://localhost:18789/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "YOUR_PASSWORD"}'

# Use the token
curl http://localhost:18789/models \
  -H "Authorization: Bearer <token>"
```

## API Endpoints

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | Public | Health check |
| `/api/auth/login` | POST | Public | Login, get JWT token |
| `/chat` | POST | Required | Chat (JSON response) |
| `/chat/stream` | GET | Required | Streaming chat (SSE) |
| `/models` | GET | Required | List available models |
| `/voice/tts` | POST | Required | Text-to-speech |
| `/voice/stt` | POST | Required | Speech-to-text |
| `/api/agents` | GET/POST | Required | Agent management |
| `/api/sessions` | GET/POST | Required | Session management |
| `/ws` | WebSocket | Public | Real-time communication |

## Configuration

Config file: `~/.openclaw-rust/config.json`

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 18789
  },
  "ai": {
    "default_provider": "ollama",
    "providers": [
      {
        "name": "ollama",
        "base_url": "http://localhost:11434",
        "default_model": "qwen3:14b"
      }
    ]
  },
  "security": {
    "jwt_secret": "your-secret-key",
    "jwt_expiration_secs": 86400,
    "admin_username": "admin",
    "admin_password_hash": "$argon2id$...",
    "cors_origins": ["*"],
    "login_rate_limit": 5,
    "api_rate_limit": 10
  }
}
```

Without `jwt_secret`, all endpoints are accessible without authentication.

## Project Structure

```
open-agentic/
├── crates/
│   ├── openagentic-core        # Core types, config, errors
│   ├── openagentic-ai          # LiteLLM unified provider (100+ LLMs)
│   ├── openagentic-agent       # Multi-agent system + skill evolution
│   ├── openagentic-server      # HTTP/WS Gateway + JWT + rate limiting
│   ├── openagentic-memory      # 3-layer memory system
│   ├── openagentic-vector      # Vector stores (Qdrant/LanceDB/Milvus)
│   ├── openagentic-channels    # Messaging integrations
│   ├── openagentic-voice       # STT/TTS services
│   ├── openagentic-canvas      # Real-time collaborative canvas
│   ├── openagentic-browser     # Browser automation
│   ├── openagentic-sandbox     # Docker/WASM sandboxing
│   ├── openagentic-tools       # Cron/Webhook/MCP tools
│   ├── openagentic-device      # Device control
│   ├── openagentic-security    # Input filter, output validation, audit
│   ├── openagentic-acp         # Agent Capability Protocol
│   ├── openagentic-ws          # WebSocket module
│   └── openagentic-cli         # CLI entry point
├── ui/                         # Web UI (React 19 + Vite + TailwindCSS)
└── android/                    # Android App (planned)
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `gateway` | Start HTTP/WebSocket server |
| `hash-password <pw>` | Generate Argon2 password hash |
| `agent` | Interactive chat mode |
| `wizard` | Interactive setup |
| `doctor` | System health check |
| `agents list/add/remove` | Manage agents |

## Roadmap

- [x] Rust Gateway with HTTP/WebSocket
- [x] LiteLLM unified provider (100+ LLMs)
- [x] JWT authentication + Argon2 password hashing
- [x] Security hardening (CORS whitelist, rate limiting, security headers)
- [ ] Ollama chat integration testing
- [ ] Web UI integration
- [ ] Android App — Phase 1: Chat MVP
- [ ] Android App — Phase 2: Screen understanding
- [ ] Android App — Phase 3: Accessibility Agent
- [ ] Public release

## Security

- **Authentication**: JWT tokens with Argon2 password hashing
- **Input protection**: Prompt injection detection (regex + keyword blacklist, multi-language)
- **Output validation**: Automatic redaction of sensitive data (API keys, passwords, credit cards)
- **Rate limiting**: Per-IP login throttling, per-token API throttling
- **CORS**: Configurable origin whitelist
- **Security headers**: X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Referrer-Policy
- **Sandboxing**: Docker and WASM isolation for tool execution

## Requirements

- **Rust**: 1.93+
- **Docker**: Optional (sandbox features)
- **Chrome/Chromium**: Optional (browser automation)

## License

MIT License — See [LICENSE](LICENSE).

---

## 中文说明

**OpenAgentic** 是一个开源 AI 手机 Agent 平台，基于 Android 无障碍服务构建。用户用自然语言描述任务，AI 自动理解屏幕内容并控制手机完成操作。

### 核心特性

- **支持 100+ 大模型** — 通过 LiteLLM 统一网关接入 OpenAI、Anthropic、Gemini、DeepSeek、通义千问、Ollama 等
- **多智能体系统** — 任务自动分解，多个专用 Agent 协作完成（研究员、程序员、写手等）
- **三层记忆** — 工作记忆 → 短期记忆（压缩摘要）→ 长期记忆（向量存储）
- **手机控制** — 通过无障碍服务读取屏幕、模拟点击/滑动/输入（开发中）
- **语音交互** — STT 语音识别 + TTS 语音合成
- **安全优先** — JWT + Argon2 认证、输入过滤、输出校验、审计日志、速率限制
- **多平台消息** — 15+ 消息通道（Telegram、Discord、钉钉、企业微信、飞书等）
- **工具生态** — 浏览器自动化、定时任务、Webhook、MCP 集成

### 快速开始

```bash
# 克隆并构建
git clone https://github.com/openagentic-ai/open-agentic.git
cd open-agentic
cargo build --release

# 启动网关
./target/release/open-agentic gateway

# 健康检查
curl http://localhost:18789/health
```

### 配置认证

```bash
# 生成密码哈希
./target/release/open-agentic hash-password 你的密码

# 编辑 ~/.openclaw-rust/config.json，填入 jwt_secret、admin_username、admin_password_hash
# 详见上方 Configuration 章节

# 登录获取 token
curl -X POST http://localhost:18789/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "你的密码"}'
```

### 开发路线

- [x] Rust 后端网关编译运行
- [x] LiteLLM 统一 Provider（支持 100+ 大模型）
- [x] JWT 认证 + Argon2 密码哈希
- [x] 安全加固（CORS 白名单、速率限制、安全响应头）
- [ ] 对接 Ollama 测试对话
- [ ] Web UI 对接后端
- [ ] Android App — 第一阶段：对话 MVP
- [ ] Android App — 第二阶段：屏幕理解
- [ ] Android App — 第三阶段：无障碍 Agent
- [ ] 产品上线

### 系统要求

- **Rust**: 1.93+
- **Docker**: 可选（沙箱功能）
- **Chrome/Chromium**: 可选（浏览器自动化）

### 许可证

MIT License — 详见 [LICENSE](LICENSE)。

---

**OpenAgentic** — 让 AI 助手更简单、更强大
