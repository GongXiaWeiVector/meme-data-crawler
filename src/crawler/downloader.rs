use crate::types::ImageMetadata;
use crate::file_manager::FileManager;
use anyhow::Result;
use sha2::{Sha256, Digest};
use chrono::Utc;
use tokio::sync::Mutex;
use std::sync::Arc;

/// 圖片下載器
#[derive(Clone)]  // 直接 derive Clone
pub struct ImageDownloader {
    file_manager: Arc<Mutex<FileManager>>,
}

impl ImageDownloader {
    pub fn new(file_manager: Arc<Mutex<FileManager>>) -> Self {
        Self { file_manager }
    }
    
    /// 下載並儲存單張圖片
    pub async fn download_and_save(
        &self,
        url: &str,
        name: &str,
        page: u32,
    ) -> Result<()> {
        // 下載圖片
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        
        // 計算 hash
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());
        
        // 生成檔名
        let ext = url.rsplit('.').next().unwrap_or("jpg");
        let filename = format!("{}_{}.{}", 
            &hash[..8], 
            sanitize_filename(name), 
            ext
        );
        
        // 建立 metadata
        let metadata = ImageMetadata {
            filename: filename.clone(),
            description: name.to_string(),
            url: url.to_string(),
            content_hash: hash,
            page_number: page,
            downloaded_at: Utc::now(),
        };
        
        // 儲存
        let fm = self.file_manager.lock().await;
        fm.save_image(&filename, &bytes)?;
        fm.append_metadata(&metadata)?;
        
        Ok(())
    }
}

/// 清理檔名
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .chars()
        .take(50)
        .collect()
}