use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::sync::mpsc;
use tauri::async_runtime::Receiver;
use futures_util::StreamExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub reading_time: u32,
    pub favicon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageMetadata {
    pub context_check: Option<String>,
    pub facts_check: Option<String>,
    pub search_check: Option<String>,
    pub reasoning: Option<String>,
    pub learning: Option<String>,
    pub search_results: Option<Vec<SearchResult>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub done: bool,
}

pub const SYSTEM_PROMPT: &str = r#"You are an AI assistant that follows a strict, structured thinking process on every response. Never deviate from this process.

PRIMARY DIRECTIVES:
1. Always analyze context before facts
2. Always check facts before searching
3. Always format responses consistently
4. Always learn from corrections
5. Never skip steps or combine them

RESPONSE STRUCTURE:
Each response must follow this exact format:

CONTEXT_CHECK:
[Previous conversation context I found relevant to this query]
[If none: "No relevant context found in our conversation"]

FACTS_CHECK:
[Facts I found in my database relevant to this query]
[If none: "No relevant facts found in database"]

SEARCH_CHECK:
[If search keywords detected: "Performing web search for: <specific_search_terms>"]
[If results available: "Found <n> relevant results:"]
[If no results: "No relevant search results found"]
[If no search needed: "No search needed for this query"]

REASONING:
[Step by step breakdown of how I'm using this information]
[Must include how I'm combining context, facts, and search]
[Must explain any conflicts between sources]

RESPONSE:
[My actual response to the user's query based on all information]

LEARNING:
[What new information should be saved to facts]
[What context was most useful]
[What searches were most helpful]"#;

#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    pub async fn chat_stream(&self, request: ChatRequest) -> Result<Receiver<String>> {
        let (tx, rx) = mpsc::channel(100);
        let client = self.client.clone();
        let url = format!("{}/api/chat", self.base_url);

        tauri::async_runtime::spawn(async move {
            let response = client
                .post(&url)
                .json(&request)
                .send()
                .await
                .unwrap();

            let mut stream = response.bytes_stream();
            let mut response_buffer = String::new();

            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                            if let Ok(response) = serde_json::from_str::<ChatResponse>(&text) {
                                if response.done {
                                    response_buffer.push_str(&response.message.content);
                                    let _ = tx.send(response_buffer.clone()).await;
                                    response_buffer.clear();
                                } else {
                                    let _ = tx.send(response.message.content).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading chunk: {:?}", e);
                    }
                }
            }
        });

        Ok(rx)
    }

    pub fn create_system_message() -> ChatMessage {
        ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
            metadata: None,
        }
    }

    pub fn create_user_message(content: String) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content,
            metadata: None,
        }
    }

    pub fn create_assistant_message(content: String) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content,
            metadata: Some(MessageMetadata {
                context_check: None,
                facts_check: None,
                search_check: None,
                reasoning: None,
                learning: None,
                search_results: None,
            }),
        }
    }
}