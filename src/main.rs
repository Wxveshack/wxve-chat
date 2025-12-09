use leptos::{
    component, create_signal, view, IntoView, SignalGet, SignalSet, SignalUpdate,
    spawn_local, mount_to_body,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

// ----------------------------------------------------------------------------
// Types - matches API contract
// ----------------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct ChatRequest {
    message: String,
    history: Vec<Message>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamChunk {
    Text { content: String },
    #[allow(dead_code)]
    ToolStart { name: String },
    #[allow(dead_code)]
    ToolEnd { name: String },
    Done,
    Error { message: String },
}

// ----------------------------------------------------------------------------
// SSE Client - POST to /chat and stream response
// ----------------------------------------------------------------------------

async fn send_message(
    message: String,
    history: Vec<Message>,
    on_chunk: impl Fn(StreamChunk) + 'static,
) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;

    let request_body = ChatRequest { message, history };
    let body_json = serde_json::to_string(&request_body).map_err(|e| e.to_string())?;

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body_json));

    let request = Request::new_with_str_and_init("https://api.wxve.io/chat", &opts)
        .map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let response: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let body = response.body().ok_or("no body")?;
    let reader = body
        .get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|e| format!("{e:?}"))?;

    let mut buffer = String::new();

    loop {
        let result = JsFuture::from(reader.read())
            .await
            .map_err(|e| format!("{e:?}"))?;

        let done = js_sys::Reflect::get(&result, &"done".into())
            .map_err(|e| format!("{e:?}"))?
            .as_bool()
            .unwrap_or(true);

        if done {
            break;
        }

        let value = js_sys::Reflect::get(&result, &"value".into())
            .map_err(|e| format!("{e:?}"))?;
        let array = js_sys::Uint8Array::new(&value);
        let mut bytes = vec![0u8; array.length() as usize];
        array.copy_to(&mut bytes);

        buffer.push_str(&String::from_utf8_lossy(&bytes));

        // Process complete SSE lines
        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer = buffer[newline_pos + 1..].to_string();

            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    let is_done = matches!(chunk, StreamChunk::Done);
                    on_chunk(chunk);
                    if is_done {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

// ----------------------------------------------------------------------------
// UI Component
// ----------------------------------------------------------------------------

#[component]
fn App() -> impl IntoView {
    let (messages, set_messages) = create_signal(Vec::<Message>::new());
    let (input, set_input) = create_signal(String::new());
    let (loading, set_loading) = create_signal(false);
    let (current_response, set_current_response) = create_signal(String::new());

    let do_send = move || {
        let msg = input.get();
        if msg.trim().is_empty() || loading.get() {
            return;
        }

        set_input.set(String::new());
        set_loading.set(true);
        set_current_response.set(String::new());

        // Add user message to history
        set_messages.update(|msgs| {
            msgs.push(Message {
                role: "user".to_string(),
                content: msg.clone(),
            });
        });

        let history = messages.get();

        spawn_local(async move {
            let result = send_message(msg, history, move |chunk| match chunk {
                StreamChunk::Text { content } => {
                    set_current_response.update(|r| r.push_str(&content));
                }
                StreamChunk::Done => {
                    let response = current_response.get();
                    set_messages.update(|msgs| {
                        msgs.push(Message {
                            role: "assistant".to_string(),
                            content: response,
                        });
                    });
                    set_current_response.set(String::new());
                    set_loading.set(false);
                }
                StreamChunk::Error { message } => {
                    set_messages.update(|msgs| {
                        msgs.push(Message {
                            role: "assistant".to_string(),
                            content: format!("Error: {message}"),
                        });
                    });
                    set_loading.set(false);
                }
                _ => {} // Ignore tool_start/tool_end for now
            })
            .await;

            if let Err(e) = result {
                set_messages.update(|msgs| {
                    msgs.push(Message {
                        role: "assistant".to_string(),
                        content: format!("Error: {e}"),
                    });
                });
                set_loading.set(false);
            }
        });
    };

    view! {
        <div>
            <div>
                {move || messages.get().iter().map(|msg| {
                    let role = msg.role.clone();
                    let content = msg.content.clone();
                    view! {
                        <div>
                            <strong>{role}": "</strong>
                            {content}
                        </div>
                    }
                }).collect::<Vec<_>>()}

                {move || {
                    let response = current_response.get();
                    if !response.is_empty() {
                        Some(view! {
                            <div>
                                <strong>"assistant: "</strong>
                                {response}
                            </div>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            <div>
                <input
                    type="text"
                    placeholder="Ask Xve..."
                    prop:value=move || input.get()
                    on:input=move |ev| {
                        set_input.set(leptos::event_target_value(&ev));
                    }
                    on:keypress=move |ev| {
                        if ev.key() == "Enter" {
                            do_send();
                        }
                    }
                />
                <button on:click=move |_| do_send() prop:disabled=move || loading.get()>
                    "Send"
                </button>
            </div>
        </div>
    }
}

// ----------------------------------------------------------------------------
// Entry point
// ----------------------------------------------------------------------------

fn main() {
    mount_to_body(|| view! { <App/> })
}
