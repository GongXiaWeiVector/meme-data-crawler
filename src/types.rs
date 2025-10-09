use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 單張圖片的 metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadata {
    /// 檔案名稱
    pub filename: String,
    /// 圖片描述
    pub description: String,
    /// 原始 URL
    pub url: String,
    /// 內容雜湊 (SHA256)
    pub content_hash: String,
    /// 來源頁面
    pub page_number: u32,
    /// 下載時間
    pub downloaded_at: DateTime<Utc>,
}

/// 爬取進度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// 最後完成的頁面
    pub last_completed_page: u32,
    /// 已下載的圖片總數
    pub total_images_downloaded: usize,
    /// 最後更新時間
    pub last_updated: DateTime<Utc>,
    /// 失敗的頁面列表
    pub failed_pages: Vec<u32>,
}

impl Progress {
    /// 建立新的進度追蹤
    pub fn new() -> Self {
        Self {
            last_completed_page: 0,
            total_images_downloaded: 0,
            last_updated: Utc::now(),
            failed_pages: Vec::new(),
        }
    }
    
    /// 更新進度
    pub fn update(&mut self, page: u32, images_count: usize) {
        self.last_completed_page = page;
        self.total_images_downloaded += images_count;
        self.last_updated = Utc::now();
    }
    
    /// 記錄失敗的頁面
    pub fn add_failed_page(&mut self, page: u32) {
        if !self.failed_pages.contains(&page) {
            self.failed_pages.push(page);
        }
        self.last_updated = Utc::now();
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

/// 重複圖片的記錄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateRecord {
    /// 內容雜湊
    pub content_hash: String,
    /// 所有具有相同雜湊的檔案
    pub files: Vec<String>,
}