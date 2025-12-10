use leptos::{
    component, create_effect, create_signal, view, For, IntoView,
    SignalGet, SignalSet, SignalUpdate, spawn_local, mount_to_body,
};
use pulldown_cmark::{html as md_html, Parser};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

fn markdown_to_html(md: &str) -> String {
    let parser = Parser::new(md);
    let mut html_output = String::new();
    md_html::push_html(&mut html_output, parser);
    html_output
}

// ----------------------------------------------------------------------------
// Types - matches API contract
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    User,
    Assistant,
}

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    #[serde(skip)]
    id: usize,
    role: Role,
    content: String,
}

#[derive(Clone, Serialize)]
struct ChatRequest {
    message: String,
    history: Vec<Message>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamChunk {
    Text { content: String },
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
    let (next_id, set_next_id) = create_signal(0usize);
    let (tool_running, set_tool_running) = create_signal::<Option<String>>(None);
    let (dark_mode, set_dark_mode) = create_signal(false);

    let toggle_dark_mode = move |_| {
        let new_value = !dark_mode.get();
        set_dark_mode.set(new_value);
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            if new_value {
                let _ = body.class_list().add_1("dark");
            } else {
                let _ = body.class_list().remove_1("dark");
            }
        }
    };

    let do_send = move || {
        let msg = input.get();
        if msg.trim().is_empty() || loading.get() {
            return;
        }

        set_input.set(String::new());
        set_loading.set(true);
        set_current_response.set(String::new());

        // Add user message to history
        let id = next_id.get();
        set_next_id.set(id + 1);
        set_messages.update(|msgs| {
            msgs.push(Message {
                id,
                role: Role::User,
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
                    let id = next_id.get();
                    set_next_id.set(id + 1);
                    set_messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: Role::Assistant,
                            content: response,
                        });
                    });
                    set_current_response.set(String::new());
                    set_loading.set(false);
                }
                StreamChunk::Error { message } => {
                    let id = next_id.get();
                    set_next_id.set(id + 1);
                    set_messages.update(|msgs| {
                        msgs.push(Message {
                            id,
                            role: Role::Assistant,
                            content: format!("Error: {message}"),
                        });
                    });
                    set_loading.set(false);
                }
                StreamChunk::ToolStart { name } => {
                    set_tool_running.set(Some(name));
                }
                StreamChunk::ToolEnd { .. } => {
                    set_tool_running.set(None);
                    set_current_response.update(|r| r.push_str("\n\n"));
                }
            })
            .await;

            if let Err(e) = result {
                let id = next_id.get();
                set_next_id.set(id + 1);
                set_messages.update(|msgs| {
                    msgs.push(Message {
                        id,
                        role: Role::Assistant,
                        content: format!("Error: {e}"),
                    });
                });
                set_loading.set(false);
            }
        });
    };

    // Auto-scroll to bottom when streaming content
    create_effect(move |_| {
        current_response.get();
        messages.get();
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.document_element() {
                    window.scroll_to_with_x_and_y(0.0, element.scroll_height() as f64);
                }
            }
        }
    });

    let has_messages = move || !messages.get().is_empty() || !current_response.get().is_empty();

    let container_class = move || {
        if has_messages() { "container has-messages" } else { "container empty" }
    };

    view! {
        <div class=container_class>
            <button
                class="theme-toggle"
                on:click=toggle_dark_mode
            >
                {move || if dark_mode.get() { "☀️" } else { "🌙" }}
            </button>
            <div class="logo">"wxve.io"</div>

            <div class="messages">
                <For
                    each=move || messages.get()
                    key=|msg| msg.id
                    children=move |msg| {
                        let class = match msg.role {
                            Role::User => "message user",
                            Role::Assistant => "message",
                        };
                        let content_html = match msg.role {
                            Role::User => msg.content.clone(),
                            Role::Assistant => markdown_to_html(&msg.content),
                        };
                        view! {
                            <div class=class>
                                <span inner_html=content_html></span>
                            </div>
                        }
                    }
                />

                {move || {
                    let response = current_response.get();
                    let tool = tool_running.get();
                    if !response.is_empty() || tool.is_some() {
                        let html = markdown_to_html(&response);
                        Some(view! {
                            <div class="message">
                                <span inner_html=html></span>
                                {move || tool_running.get().map(|name| view! {
                                    <div class="tool-indicator">
                                        <span class="spinner"></span>
                                        {format!("Using {name}...")}
                                    </div>
                                })}
                            </div>
                        })
                    } else {
                        None
                    }
                }}
            </div>

            <div class="input-area">
                <div class="input-box">
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
        </div>
    }
}

// ----------------------------------------------------------------------------
// Entry point
// ----------------------------------------------------------------------------

fn main() {
    mount_to_body(|| view! { <App/> })
}
