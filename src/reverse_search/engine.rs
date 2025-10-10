use crate::file_manager::FileManager;
use super::{
    trait_def::ReverseSearchService,
    types::{ReverseSearchResult, SearchProgress},
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::Duration;
use std::fs;
use std::path::Path;

pub struct ReverseSearchEngine {
    file_manager: FileManager,
    services: Vec<Arc<dyn ReverseSearchService>>,
    concurrency: usize,
    progress_file: String,
    results_file: String,
}

impl ReverseSearchEngine {
    pub fn new(
        data_dir: &str,
        services: Vec<Arc<dyn ReverseSearchService>>,
        concurrency: usize,
    ) -> Result<Self> {
        Ok(Self {
            file_manager: FileManager::new(data_dir)?,
            services,
            concurrency,
            progress_file: format!("{}/search_progress.json", data_dir),
            results_file: format!("{}/reverse_search_results.jsonl", data_dir),
        })
    }
    
    pub fn load_progress(&self) -> Result<SearchProgress> {
        if !Path::new(&self.progress_file).exists() {
            return Ok(SearchProgress::new());
        }
        
        let content = fs::read_to_string(&self.progress_file)?;
        Ok(serde_json::from_str(&content)?)
    }
    
    pub fn save_progress(&self, progress: &SearchProgress) -> Result<()> {
        let temp_path = format!("{}.tmp", self.progress_file);
        let json = serde_json::to_string_pretty(progress)?;
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.progress_file)?;
        Ok(())
    }
    
    pub fn append_result(&self, result: &ReverseSearchResult) -> Result<()> {
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.results_file)?;
        
        writeln!(file, "{}", serde_json::to_string(result)?)?;
        Ok(())
    }
    
    pub async fn run(&self) -> Result<()> {
        println!("📖 讀取圖片列表...");
        let all_metadata = self.file_manager.load_all_metadata()?;
        
        println!("📋 載入進度...");
        let mut progress = self.load_progress()?;
        
        let pending: Vec<_> = all_metadata
            .into_iter()
            .filter(|m| !progress.is_completed(&m.filename))
            .collect();
        
        if pending.is_empty() {
            println!("✅ 所有圖片都已搜尋完成！");
            return Ok(());
        }
        
        println!("🔍 待搜尋: {} 張 (已完成: {})", 
            pending.len(), 
            progress.completed_files.len()
        );
        
        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        
        for (idx, metadata) in pending.iter().enumerate() {
            println!("[{}/{}] 搜尋: {}", 
                idx + 1, 
                pending.len(), 
                metadata.filename
            );
            
            for service in &self.services {
                let _permit = semaphore.acquire().await?;
                
                println!("  🔎 使用 {} 搜尋...", service.name());
                
                match service.search(metadata).await {
                    Ok(result) => {
                        println!("    ✅ 找到 {} 個關鍵字", result.keywords.len());
                        self.append_result(&result)?;
                    }
                    Err(e) => {
                        eprintln!("    ❌ 失敗: {}", e);
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(
                    service.suggested_delay_ms()
                )).await;
            }
            
            progress.add_completed(metadata.filename.clone());
            self.save_progress(&progress)?;
            
            if (idx + 1) % 10 == 0 {
                println!("💾 已處理 {} 張\n", idx + 1);
            }
        }
        
        println!("\n✅ 全部完成！");
        Ok(())
    }
}