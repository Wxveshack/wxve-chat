# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Open-source chat interface for Xve (chat.wxve.io), the analytical voice of Wxve.

## Commands

```bash
# Dev server with hot reload
trunk serve

# Production build (output in dist/)
trunk build --release

# Lint
cargo clippy --target wasm32-unknown-unknown
```

## API Contract

**Endpoint:** `POST https://api.wxve.io/chat`

**Request:**
```json
{
  "message": "What's the wave structure for AMZN?",
  "history": [
    {"role": "user", "content": "previous message"},
    {"role": "assistant", "content": "previous response"}
  ]
}
```

**Response:** SSE stream (`text/event-stream`)

```
data: {"type": "text", "content": "AMZN"}
data: {"type": "text", "content": " is"}
data: {"type": "tool_start", "name": "getSecurityStructures"}
data: {"type": "tool_end", "name": "getSecurityStructures"}
data: {"type": "text", "content": " in wave 3..."}
data: {"type": "done"}
```

**Chunk types:**
- `text` - Token from Xve (stream to UI)
- `tool_start` - Xve is calling a tool (show indicator)
- `tool_end` - Tool completed
- `done` - Response complete
- `error` - Something went wrong

## Architecture

Single-file Leptos app (`src/main.rs`) with three sections:
1. **Types** - `ChatRequest`, `Message`, `StreamChunk` (serde-tagged enum matching API)
2. **SSE Client** - `send_message()` async fn using web-sys fetch + ReadableStream
3. **UI Component** - `App` component with signals for messages, input, loading state

Compiles to WASM via Trunk. Deployed as static files to S3 + CloudFront.

## Code Style

- Use explicit imports (no `use leptos::*`)
- Keep everything in `main.rs` until complexity demands otherwise
