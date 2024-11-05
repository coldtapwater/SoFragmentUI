use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub reading_time: u32,
    pub favicon_url: Option<String>,
    pub is_paywall: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub max_results: usize,
}

#[derive(Clone)]
pub struct SearchClient {
    client: Client,
    base_url: String,
}

impl SearchClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            base_url: "https://duckduckgo.com/html".to_string(),
        }
    }

    async fn extract_content(&self, url: &str) -> Result<Option<String>> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Ok(None);
        }

        let text = response.text().await?;
        
        // Move HTML parsing to a blocking task to avoid Send issues
        let content = tokio::task::spawn_blocking(move || {
            let document = Html::parse_document(&text);
            
            // Define selectors here to avoid Send issues
            let paywall_selectors = [
                ".paywall", "#paywall", ".subscribe-wall",
                ".subscription-required", ".paid-content",
            ];
            
            for selector in paywall_selectors {
                if let Ok(sel) = Selector::parse(selector) {
                    if document.select(&sel).next().is_some() {
                        return None;
                    }
                }
            }

            let content_selectors = [
                "article", ".article-content", ".post-content",
                "main", "[role='main']", ".content",
            ];

            for selector in content_selectors {
                if let Ok(sel) = Selector::parse(selector) {
                    if let Some(element) = document.select(&sel).next() {
                        return Some(element.text().collect::<Vec<_>>().join(" "));
                    }
                }
            }

            // Fallback
            Some(document.select(&Selector::parse("body").unwrap_or_else(|_| Selector::parse("html").unwrap()))
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_default())
        }).await.unwrap_or(None);

        Ok(content)
    }

    async fn process_search_results(&self, html: String, max_results: usize) -> Vec<SearchResult> {
        let document = Html::parse_document(&html);
        let mut results = Vec::new();
        
        // Move selector parsing outside of async context
        let result_selector = Selector::parse(".result").unwrap();
        let link_selector = Selector::parse(".result__a").unwrap();

        for result in document.select(&result_selector).take(max_results) {
            if let Some(link) = result.select(&link_selector).next() {
                let url = link.value().attr("href").unwrap_or_default();
                let title = link.text().collect::<String>();

                if let Ok(Some(content)) = self.extract_content(url).await {
                    let reading_time = (content.split_whitespace().count() as u32 / 100).max(1);
                    let summary = Self::generate_summary(&content);
                    let favicon_url = Self::get_favicon_url(url);

                    results.push(SearchResult {
                        url: url.to_string(),
                        title,
                        summary,
                        reading_time,
                        favicon_url,
                        is_paywall: false,
                    });
                }
            }
        }
        
        results
    }

    pub async fn search_stream(&self, request: SearchRequest) -> Result<mpsc::Receiver<SearchResult>> {
        let (tx, rx) = mpsc::channel(100);
        let query = request.query.clone();
        let max_results = request.max_results;
        let client = self.client.clone();
        let base_url = self.base_url.clone();
    
        // Make the HTTP request outside the blocking task
        let response = client
            .post(&base_url)
            .form(&[
                ("q", query.as_str()),
                ("kl", "us-en"),
            ])
            .send()
            .await?
            .text()
            .await?;
    
        // Process HTML in a blocking task
        let tx_clone = tx.clone();
        tokio::task::spawn_blocking(move || {
            let document = Html::parse_document(&response);
            if let Ok(result_selector) = Selector::parse(".result") {
                if let Ok(link_selector) = Selector::parse(".result__a") {
                    for result in document.select(&result_selector).take(max_results) {
                        if let Some(link) = result.select(&link_selector).next() {
                            if let Some(url) = link.value().attr("href") {
                                let title = link.text().collect::<String>();
                                let search_result = SearchResult {
                                    url: url.to_string(),
                                    title,
                                    summary: String::new(),
                                    reading_time: 0,
                                    favicon_url: None,
                                    is_paywall: false,
                                };
                                
                                // Use blocking_send since we're in a blocking task
                                if tx_clone.blocking_send(search_result).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
    
        Ok(rx)
    }

    fn get_favicon_url(url: &str) -> Option<String> {
        Url::parse(url).ok().map(|parsed_url| {
            format!(
                "{}://{}/favicon.ico",
                parsed_url.scheme(),
                parsed_url.host_str().unwrap_or_default()
            )
        })
    }

    fn generate_summary(content: &str) -> String {
        let words: Vec<&str> = content.split_whitespace().collect();
        let summary: String = words.iter().take(50).cloned().collect::<Vec<_>>().join(" ");
        if words.len() > 50 {
            summary + "..."
        } else {
            summary
        }
    }
}