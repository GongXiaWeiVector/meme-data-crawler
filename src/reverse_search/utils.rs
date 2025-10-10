use scraper::{Html, Selector};

/// 從 Google 結果提取 "Best guess"
pub fn extract_best_guess(document: &Html) -> Option<String> {
    let selectors = vec![
        "div[data-async-context] a",
        "a.fKDtNb",
        "div.i3LlFf a",
    ];
    
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(elem) = document.select(&selector).next() {
                if let Some(text) = elem.text().next() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }
    
    None
}

/// 提取關鍵字
pub fn extract_keywords(document: &Html) -> Vec<String> {
    let mut keywords = Vec::new();
    
    // 從標題提取
    if let Ok(selector) = Selector::parse("title") {
        if let Some(title) = document.select(&selector).next() {
            if let Some(text) = title.text().next() {
                keywords.extend(
                    text.split(&[' ', '-', '|', ','][..])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                );
            }
        }
    }
    
    // 去重
    keywords.sort();
    keywords.dedup();
    
    keywords
}

/// 提取相關網站
pub fn extract_related_sites(document: &Html) -> Vec<String> {
    let mut sites = Vec::new();
    
    if let Ok(selector) = Selector::parse("a[href]") {
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