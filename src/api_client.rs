use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool, // 虽然是同步线程，我们依然可以用流式处理
}

pub struct ApiClient {
    client: Client,
    api_key: String,
    api_base: String,
    model: String,
}

impl ApiClient {
    pub fn new(config: &crate::Config) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", config.api_key).parse().unwrap(),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();

        Self {
            client,
            api_key: config.api_key.clone(),
            api_base: config.api_base.clone(),
            model: config.model.clone(),
        }
    }

    pub fn send_chat_stream(&self, messages: Vec<ChatMessage>, tx: std::sync::mpsc::Sender<crate::AppMessage>) {
        let url = format!("{}", self.api_base);
        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
        };

        let response = self.client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send();

        match response {
            Ok(res) => {
                // 关键点：使用读取器处理 SSE 流
                let reader = std::io::BufReader::new(res);
                use std::io::BufRead;

                for line in reader.lines() {
                    let line = line.unwrap_or_default();
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" { break; }

                        // 解析 JSON 提取文本片段 (Chunk)
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                // 通过通道传回主线程
                                tx.send(crate::AppMessage::ModelChunk(content.to_string())).unwrap();
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tx.send(crate::AppMessage::SystemLog(format!("网络请求失败: {}", e))).unwrap();
            }
        }
        tx.send(crate::AppMessage::TaskComplete).unwrap();
    }
}