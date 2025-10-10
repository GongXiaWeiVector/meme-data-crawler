use crate::types::ImageMetadata;
use crate::reverse_search::{
    trait_def::ReverseSearchService,
    types::{ReverseSearchResult, KeywordFilter},
    utils,
};
use anyhow::Result;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT, ACCEPT_LANGUAGE};

pub struct GoogleUrlService {
    client: reqwest::Client,
    filter: KeywordFilter,
}

impl GoogleUrlService {
    pub fn new(filter: KeywordFilter) -> Result<Self> {
        // 建立更真實的 headers
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
        ));
        headers.insert(ACCEPT, HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
        ));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static(
            "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7"
        ));
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .cookie_store(true)  // 啟用 cookie
            .build()?;
        
        Ok(Self { client, filter })
    }
}

#[async_trait::async_trait]
impl ReverseSearchService for GoogleUrlService {
    fn name(&self) -> &str {
        "google"
    }
    
    async fn search(&self, metadata: &ImageMetadata) -> Result<ReverseSearchResult> {
        // 檢查是否為 404 頁面
        let html = self.fetch_with_retry(&metadata.url).await?;
        
        // 如果是 404 頁面，提早回傳
        if html.contains("404") && html.contains("Error") {
            return Ok(ReverseSearchResult {
                filename: metadata.filename.clone(),
                service: self.name().to_string(),
                suggested_title: Some("⚠️ Google blocked request".to_string()),
                keywords: vec!["blocked".to_string()],
                related_sites: vec![],
                best_guess: None,
                searched_at: chrono::Utc::now(),
            });
        }
        
        let document = scraper::Html::parse_document(&html);
        
        let best_guess = utils::extract_best_guess(&document);
        let mut keywords = utils::extract_keywords(&document);
        keywords = self.filter.filter(keywords);
        let related_sites = utils::extract_related_sites(&document);
        
        Ok(ReverseSearchResult {
            filename: metadata.filename.clone(),
            service: self.name().to_string(),
            suggested_title: best_guess.clone(),
            keywords,
            related_sites,
            best_guess,
            searched_at: chrono::Utc::now(),
        })
    }
    
    fn suggested_delay_ms(&self) -> u64 {
        5000  // 增加到 5 秒延遲
    }
}

impl GoogleUrlService {
    async fn fetch_with_retry(&self, image_url: &str) -> Result<String> {
        let search_url = format!(
            "https://www.google.com/searchbyimage?image_url={}",
            urlencoding::encode(image_url)
        );
        
        // 重試最多 3 次
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
            }
            
            match self.client.get(&search_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(html) => return Ok(html),
                            Err(e) => {
                                eprintln!("    讀取回應失敗 (嘗試 {}): {}", attempt + 1, e);
                                continue;
                            }
                        }
                    } else {
                        eprintln!("    HTTP 錯誤 {}: 嘗試 {}", response.status(), attempt + 1);
                        continue;
                    }
                }
                Err(e) => {
                    eprintln!("    請求失敗 (嘗試 {}): {}", attempt + 1, e);
                    continue;
                }
            }
        }
        
        Err(anyhow::anyhow!("多次嘗試後仍失敗"))
    }
}