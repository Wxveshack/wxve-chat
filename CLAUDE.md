# CLAUDE.md

## Project

Open-source chat interface for Xve (chat.wxve.io), the analytical voice of Wxve.

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

## Stack

**Framework:** Leptos (Rust → WASM)

**Build:** Trunk (WASM bundler)

**Deployment:** Static WASM + HTML/CSS/JS to S3 + CloudFront at chat.wxve.io

**Key Dependencies:**
- `leptos` - Reactive UI framework
- `gloo-net` - Fetch/SSE for WASM
- `serde` / `serde_json` - Serialization (shared types with backend)
- `web-sys` - Web API bindings (localStorage, etc.)

## Project Structure

```
wxve-chat/
├── Cargo.toml
├── Trunk.toml
├── index.html
├── src/
│   ├── main.rs          # App entry, mount root component
│   ├── app.rs           # Root <App/> component
│   ├── components/
│   │   ├── mod.rs
│   │   ├── chat.rs      # Chat container
│   │   ├── message.rs   # Individual message bubble
│   │   └── input.rs     # Message input box
│   ├── api/
│   │   ├── mod.rs
│   │   ├── types.rs     # ChatRequest, StreamChunk enum
│   │   └── client.rs    # SSE streaming client
│   └── state/
│       ├── mod.rs
│       └── conversation.rs  # Conversation history, localStorage
└── styles/
    └── main.css
```

## Commands

```bash
# Dev server with hot reload
trunk serve

# Production build
trunk build --release

# Output in dist/ → deploy to S3
```

## Design

Minimal, fast, accessible. Xve speaks - the UI should stay out of the way.

## Future Considerations

- **Download conversation:** Serialize history to JSON/Markdown via web-sys Blob API
- **Visualizations:** Canvas/WebGL bindings in Rust, or integrate with charting libs via JS interop
