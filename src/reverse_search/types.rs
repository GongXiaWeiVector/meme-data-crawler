use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use chrono::{DateTime, Utc};

/// 反向搜尋結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseSearchResult {
    pub filename: String,
    pub service: String,
    pub suggested_title: Option<String>,
    pub keywords: Vec<String>,
    pub related_sites: Vec<String>,
    pub best_guess: Option<String>,
    pub searched_at: DateTime<Utc>,
}

/// 搜尋進度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProgress {
    pub completed_files: HashSet<String>,
    pub last_updated: DateTime<Utc>,
}

impl SearchProgress {
    pub fn new() -> Self {
        Self {
            completed_files: HashSet::new(),
            last_updated: Utc::now(),
        }
    }
    
    pub fn add_completed(&mut self, filename: String) {
        self.completed_files.insert(filename);
        self.last_updated = Utc::now();
    }
    
    pub fn is_completed(&self, filename: &str) -> bool {
        self.completed_files.contains(filename)
    }
}

impl Default for SearchProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// 關鍵字過濾器
#[derive(Debug, Clone)]
pub struct KeywordFilter {
    pub blocklist: Vec<String>,
    pub allowlist: Vec<String>,
    pub min_length: usize,
}

impl Default for KeywordFilter {
    fn default() -> Self {
        Self {
            blocklist: vec![
                "porn".to_string(),
                "xxx".to_string(),
                "adult".to_string(),
            ],
            allowlist: vec![],
            min_length: 3,
        }
    }
}

impl KeywordFilter {
    pub fn filter(&self, keywords: Vec<String>) -> Vec<String> {
        keywords
            .into_iter()
            .filter(|kw| {
                if kw.len() < self.min_length {
                    return false;
                }
                
                let kw_lower = kw.to_lowercase();
                if self.blocklist.iter().any(|blocked| kw_lower.contains(blocked)) {
                    return false;
                }
                
                if !self.allowlist.is_empty() {
                    return self.allowlist.iter().any(|allowed| {
                        kw_lower.contains(&allowed.to_lowercase())
                    });
                }
                
                true
            })
            .collect()
    }
}