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
- `serde` / `serde_json` - Serialization
- `web-sys` - Web API bindings (fetch, streams)
- `wasm-bindgen` / `wasm-bindgen-futures` - JS interop and async

## Project Structure

```
wxve-chat/
├── Cargo.toml
├── index.html
├── src/
│   └── main.rs    # Everything: types, SSE client, UI, main()
└── styles/
    └── main.css   # (future)
```

Single file until complexity demands otherwise.

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
