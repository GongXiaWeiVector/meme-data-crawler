use crate::types::ImageMetadata;
use crate::reverse_search::{
    trait_def::ReverseSearchService,
    types::ReverseSearchResult,
};
use anyhow::Result;
use std::time::Duration;
use scraper::{Html, Selector};

pub struct TinEyeService {
    client: reqwest::Client,
}

impl TinEyeService {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;
        
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl ReverseSearchService for TinEyeService {
    fn name(&self) -> &str {
        "tineye"
    }
    
    async fn search(&self, metadata: &ImageMetadata) -> Result<ReverseSearchResult> {
        let search_url = format!(
            "https://tineye.com/search?url={}",
            urlencoding::encode(&metadata.url)
        );
        
        let html = self.client
            .get(&search_url)
            .send()
            .await?
            .text()
            .await?;
        
        let document = Html::parse_document(&html);
        
        let match_count = extract_match_count(&document);
        let related_sites = extract_related_sites(&document);
        let title = extract_title(&document);
        
        let mut keywords = vec![];
        if match_count > 0 {
            keywords.push(format!("{} matches", match_count));
        }
        
        Ok(ReverseSearchResult {
            filename: metadata.filename.clone(),
            service: self.name().to_string(),
            suggested_title: title,
            keywords,
            related_sites,
            best_guess: None,
            searched_at: chrono::Utc::now(),
        })
    }
    
    fn suggested_delay_ms(&self) -> u64 {
        3000
    }
}

fn extract_match_count(document: &Html) -> usize {
    let selectors = vec![
        ".matches-count",
        ".result-count",
        "h2.search-results-message",
    ];
    
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(elem) = document.select(&selector).next() {
                let text = elem.text().collect::<String>();
                
                for word in text.split_whitespace() {
                    let cleaned = word.replace(',', "").replace(".", "");
                    if let Ok(num) = cleaned.parse::<usize>() {
                        return num;
                    }
                }
            }
        }
    }
    
    0
}

fn extract_related_sites(document: &Html) -> Vec<String> {
    let mut sites = Vec::new();
    
    if let Ok(selector) = Selector::parse(".match-thumb a") {
        for elem in document.select(&selector).take(10) {
            if let Some(href) = elem.value().attr("href") {
                if href.starts_with("http") {
                    sites.push(href.to_string());
                }
            }
        }
    }
    
    sites
}

fn extract_title(document: &Html) -> Option<String> {
    if let Ok(selector) = Selector::parse("title") {
        if let Some(title) = document.select(&selector).next() {
            let text = title.text().collect::<String>();
            if !text.contains("TinEye") && !text.is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }
    
    None
}