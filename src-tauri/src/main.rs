#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod ollama;
mod search;
use tauri::Emitter;
use ollama::{ChatMessage, ChatRequest, OllamaClient, SYSTEM_PROMPT};
use tauri::State;
use tokio::sync::Mutex;
use crate::search::{SearchClient, SearchRequest, SearchResult};

// State management for conversation context
struct ConversationState {
    messages: Vec<ChatMessage>,
}

struct SearchState {
    client: SearchClient,
}

// Combined state management
struct AppState {
    ollama: Mutex<OllamaClient>,
    conversation: Mutex<ConversationState>,
    search: Mutex<SearchState>,
}

#[tauri::command]
async fn perform_search(
    window: tauri::Window,
    query: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Clone what we need before spawning
    let search_client = {
        let search_state = state.search.lock().await;
        search_state.client.clone()
    };

    let request = SearchRequest {
        query,
        max_results: 5,
    };

    // Use cloned client instead of state reference
    let mut receiver = search_client
        .search_stream(request)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(result) = receiver.recv().await {
        window.emit("search-result", &result)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn chat_stream(
    window: tauri::Window,
    message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut conversation = state.conversation.lock().await;
    
    // Create new user message
    let user_message = OllamaClient::create_user_message(message);
    
    // Build messages array starting with system prompt
    let mut messages = vec![
        OllamaClient::create_system_message(),
    ];

    // Add relevant conversation history
    // We'll take the last few messages to maintain context
    let history_start = conversation.messages.len().saturating_sub(5);
    messages.extend(conversation.messages[history_start..].iter().cloned());
    
    // Add the new user message
    messages.push(user_message.clone());

    // Create request with full context in messages
    let request = ChatRequest {
        model: "granite3-moe".to_string(),
        messages,
        stream: true,
    };

    // Get client and send request
    let client = {
        let client = state.ollama.lock().await;
        client.clone()
    };

    // Add user message to conversation history
    conversation.messages.push(user_message);

    let mut receiver = client
        .chat_stream(request)
        .await
        .map_err(|e| e.to_string())?;

    drop(conversation); // Release the lock before entering the loop

    let mut complete_message = String::new();

    while let Some(chunk) = receiver.recv().await {
        window
            .emit("chat-response", &chunk)
            .map_err(|e| e.to_string())?;
        complete_message.push_str(&chunk);
    }

    // Once streaming is complete, add assistant's response to conversation history
    if !complete_message.is_empty() {
        let mut conversation = state.conversation.lock().await; // Re-acquire the lock
        let context_len = conversation.messages.len();
        
        let assistant_message = OllamaClient::create_assistant_message(complete_message);
        
        if context_len > 10 {
            conversation.messages.drain(0..context_len - 10);
        }
        
        conversation.messages.push(assistant_message);
    }

    Ok(())
}

#[tauri::command]
async fn clear_conversation(state: State<'_, AppState>) -> Result<(), String> {
    let mut conversation = state.conversation.lock().await;
    conversation.messages.clear();
    Ok(())
}

fn main() {
    let app_state = AppState {
        ollama: Mutex::new(OllamaClient::new()),
        conversation: Mutex::new(ConversationState {
            messages: Vec::new(),
        }),
        search: Mutex::new(SearchState {
            client: SearchClient::new(),
        }),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            chat_stream,
            clear_conversation,
            perform_search
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
