use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::{get_model, AssistantMessage, SystemMessage};

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
    reasoning_model: String,
    coder_model: String
}

#[derive(Clone)]
pub enum Model {
    Reasoning,
    Coder
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
            reasoning_model: config.reasoning_model.clone(),
            coder_model: config.coder_model.clone(),
        }
    }

    pub fn send_chat_stream(&self, messages: Vec<ChatMessage>, tx: std::sync::mpsc::Sender<crate::AppMessage>) {
        let url = format!("{}", self.api_base);

        let model = get_model().read().unwrap().clone();
        let model = match model {
            Model::Reasoning => self.reasoning_model.clone(),
            Model::Coder => self.coder_model.clone()
        };

        let request_body = ChatRequest {
            model,
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
                                let _ = tx.send(crate::AppMessage::AIMsg(AssistantMessage::ModelChunk(content.to_string())));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("网络请求失败: {}", e);
                // 简单过滤潜在的敏感信息
                let safe_msg = if error_msg.contains("Bearer ") || error_msg.contains("api_key") {
                    "网络请求失败: 认证错误或网络问题".to_string()
                } else {
                    error_msg
                };
                let _ = tx.send(crate::AppMessage::SysMsg(SystemMessage::SystemLog(safe_msg)));
            }
        }
        let _ = tx.send(crate::AppMessage::AIMsg(AssistantMessage::TaskComplete));
    }
}