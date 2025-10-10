use crate::types::ImageMetadata;
use crate::reverse_search::{
    trait_def::ReverseSearchService,
    types::{ReverseSearchResult, KeywordFilter},
};
use anyhow::Result;
use std::time::Duration;
use scraper::{Html, Selector};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, ACCEPT};

pub struct BingService {
    client: reqwest::Client,
    filter: KeywordFilter,
}

impl BingService {
    pub fn new(filter: KeywordFilter) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        ));
        headers.insert(ACCEPT, HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
        ));
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()?;
        
        Ok(Self { client, filter })
    }
}

#[async_trait::async_trait]
impl ReverseSearchService for BingService {
    fn name(&self) -> &str {
        "bing"
    }
    
    async fn search(&self, metadata: &ImageMetadata) -> Result<ReverseSearchResult> {
        let search_url = format!(
            "https://www.bing.com/images/search?view=detailv2&iss=sbi&q=imgurl:{}",
            urlencoding::encode(&metadata.url)
        );
        
        let html = self.client
            .get(&search_url)
            .send()
            .await?
            .text()
            .await?;
        
        let document = Html::parse_document(&html);
        
        let best_guess = extract_best_guess(&document);
        let mut keywords = extract_keywords(&document);
        keywords = self.filter.filter(keywords);
        let related_sites = extract_related_sites(&document);
        
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
        4000
    }
}

fn extract_best_guess(document: &Html) -> Option<String> {
    let selectors = vec![
        "h2.bestRepresentativeQuery a",
        ".bestRepresentativeQuery",
        "div.caption a",
    ];
    
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(elem) = document.select(&selector).next() {
                let text = elem.text().collect::<String>();
                if !text.is_empty() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }
    
    None
}

fn extract_keywords(document: &Html) -> Vec<String> {
    let mut keywords = Vec::new();
    
    if let Ok(selector) = Selector::parse("meta[name='keywords']") {
        if let Some(elem) = document.select(&selector).next() {
            if let Some(content) = elem.value().attr("content") {
                keywords.extend(
                    content.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                );
            }
        }
    }
    
    if let Ok(selector) = Selector::parse(".rms a") {
        for elem in document.select(&selector).take(10) {
            let text = elem.text().collect::<String>();
            if !text.is_empty() {
                keywords.push(text.trim().to_string());
            }
        }
    }
    
    if let Ok(selector) = Selector::parse("title") {
        if let Some(title) = document.select(&selector).next() {
            let text = title.text().collect::<String>();
            keywords.extend(
                text.split(&[' ', '-', '|'][..])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && s.len() > 2)
            );
        }
    }
    
    keywords.sort();
    keywords.dedup();
    
    keywords
}

fn extract_related_sites(document: &Html) -> Vec<String> {
    let mut sites = Vec::new();
    
    if let Ok(selector) = Selector::parse("a.iusc") {
        for elem in document.select(&selector).take(10) {
            if let Some(m) = elem.value().attr("m") {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(m) {
                    if let Some(purl) = json["purl"].as_str() {
                        sites.push(purl.to_string());
                    }
                }
            }
        }
    }
    
    sites
}