use crate::types::ImageMetadata;
use crate::reverse_search::{
    trait_def::ReverseSearchService,
    types::ReverseSearchResult,
};
use anyhow::Result;

pub struct GoogleVisionService {
    api_key: String,
    client: reqwest::Client,
}

impl GoogleVisionService {
    pub fn new(api_key: String) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { api_key, client })
    }
}

#[async_trait::async_trait]
impl ReverseSearchService for GoogleVisionService {
    fn name(&self) -> &str {
        "google-vision"
    }
    
    async fn search(&self, metadata: &ImageMetadata) -> Result<ReverseSearchResult> {
        // Google Vision API 呼叫
        let api_url = format!(
            "https://vision.googleapis.com/v1/images:annotate?key={}",
            self.api_key
        );
        
        let request_body = serde_json::json!({
            "requests": [{
                "image": {
                    "source": {
                        "imageUri": metadata.url
                    }
                },
                "features": [
                    {"type": "WEB_DETECTION"},
                    {"type": "LABEL_DETECTION"}
                ]
            }]
        });
        
        let response = self.client
            .post(&api_url)
            .json(&request_body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        
        // 解析結果
        let keywords = extract_labels(&response);
        let related_sites = extract_web_entities(&response);
        
        Ok(ReverseSearchResult {
            filename: metadata.filename.clone(),
            service: self.name().to_string(),
            suggested_title: None,
            keywords,
            related_sites,
            best_guess: None,
            searched_at: chrono::Utc::now(),
        })
    }
    
    fn requires_api_key(&self) -> bool {
        true
    }
    
    fn suggested_delay_ms(&self) -> u64 {
        500  // API 通常可以更快
    }
}

fn extract_labels(response: &serde_json::Value) -> Vec<String> {
    response["responses"][0]["labelAnnotations"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|label| label["description"].as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn extract_web_entities(response: &serde_json::Value) -> Vec<String> {
    response["responses"][0]["webDetection"]["webEntities"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|entity| entity["description"].as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}