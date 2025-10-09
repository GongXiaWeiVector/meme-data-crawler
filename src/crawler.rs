use crate::types::{ImageMetadata, Progress};
use crate::file_manager::FileManager;
use crate::fetcher::{Fetcher, HttpFetcher};
use crate::parser::PageParser;
use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::{Semaphore, Mutex};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

/// 爬蟲配置
pub struct CrawlerConfig {
    /// 並發數量
    pub concurrency: usize,
    /// 請求超時（秒）
    pub timeout_secs: u64,
    /// 最大重試次數
    pub max_retries: u32,
    /// 每批次間隔（毫秒）
    pub batch_delay_ms: u64,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            timeout_secs: 30,
            max_retries: 3,
            batch_delay_ms: 1000,
        }
    }
}

/// 主爬蟲
pub struct Crawler {
    file_manager: Arc<Mutex<FileManager>>,
    fetcher: Arc<HttpFetcher>,
    parser: Arc<dyn PageParser>,
    base_url: String,
    total_pages: u32,
    config: CrawlerConfig,
}

impl Crawler {
    /// 建立新的爬蟲
    pub fn new(
        data_dir: &str,
        base_url: String,
        total_pages: u32,
        parser: Arc<dyn PageParser>,
        config: CrawlerConfig,
    ) -> Result<Self> {
        let file_manager = Arc::new(Mutex::new(FileManager::new(data_dir)?));
        let fetcher = Arc::new(HttpFetcher::new(config.timeout_secs, config.max_retries)?);
        
        Ok(Self {
            file_manager,
            fetcher,
            parser,
            base_url,
            total_pages,
            config,
        })
    }
    
    /// 執行爬蟲
    pub async fn run(&self) -> Result<()> {
        println!("載入進度...");
        let mut progress = self.file_manager.lock().await.load_progress()?;
        
        let start_page = progress.last_completed_page + 1;
        println!("從第 {} 頁開始爬取", start_page);
        println!("並發數: {}", self.config.concurrency);
        println!("總頁數: {}\n", self.total_pages);
        
        // 建立多進度條管理器
        let multi_progress = MultiProgress::new();
        
        // 主進度條（頁面）
        let main_pb = multi_progress.add(ProgressBar::new(self.total_pages as u64));
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} 頁 ({percent}%) {eta}")
                .unwrap()
                .progress_chars("=>-")
        );
        main_pb.set_message("📄 頁面進度");
        main_pb.set_position(progress.last_completed_page as u64);
        
        // 圖片進度條
        let image_pb = multi_progress.add(ProgressBar::new(0));
        image_pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {pos} 張")
                .unwrap()
        );
        image_pb.set_message("🖼️  已下載圖片:");
        image_pb.set_position(progress.total_images_downloaded as u64);
        
        // 狀態進度條（顯示當前處理的頁面）
        let status_pb = multi_progress.add(ProgressBar::new(0));
        status_pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg}")
                .unwrap()
        );
        
        // 建立並發控制
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));
        let progress_mutex = Arc::new(Mutex::new(progress));
        
        // 分批處理
        for batch_start in (start_page..=self.total_pages).step_by(self.config.concurrency) {
            let batch_end = (batch_start + self.config.concurrency as u32 - 1)
                .min(self.total_pages);
            
            status_pb.set_message(format!("⚡ 正在處理: 第 {} - {} 頁", batch_start, batch_end));
            
            let mut tasks = vec![];
            
            // 建立批次任務
            for page in batch_start..=batch_end {
                let semaphore = Arc::clone(&semaphore);
                let fetcher = Arc::clone(&self.fetcher);
                let parser = Arc::clone(&self.parser);
                let file_manager = Arc::clone(&self.file_manager);
                let progress_mutex = Arc::clone(&progress_mutex);
                let base_url = self.base_url.clone();
                let main_pb = main_pb.clone();
                let image_pb = image_pb.clone();
                let status_pb = status_pb.clone();
                
                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    
                    status_pb.set_message(format!("🔄 爬取第 {} 頁...", page));
                    
                    let url = format!("{}?page={}", base_url, page);
                    let html_result = fetcher.fetch_page(&url).await;
                    
                    let result = match html_result {
                        Ok(html) => {
                            // 解析頁面
                            match parser.parse_page(&html) {
                                Ok(images) => {
                                    let count = images.len();
                                    status_pb.set_message(format!("📥 第 {} 頁: 找到 {} 張圖片", page, count));
                                    
                                    // 下載圖片
                                    let mut success_count = 0;
                                    for (url, name) in images {
                                        match Self::download_and_save_image_static(
                                            &file_manager,
                                            &url,
                                            &name,
                                            page
                                        ).await {
                                            Ok(_) => {
                                                success_count += 1;
                                                image_pb.inc(1);
                                            }
                                            Err(e) => {
                                                eprintln!("下載失敗 ({}): {}", name, e);
                                            }
                                        }
                                    }
                                    
                                    Ok(success_count)
                                }
                                Err(e) => Err(e),
                            }
                        }
                        Err(e) => Err(e),
                    };
                    
                    main_pb.inc(1);
                    (page, result)
                });
                
                tasks.push(task);
            }
            
            // 等待批次完成
            for task in tasks {
                let (page, result) = task.await.unwrap();
                
                let mut progress = progress_mutex.lock().await;
                
                match result {
                    Ok(count) => {
                        progress.update(page, count);
                        status_pb.set_message(format!("✅ 第 {} 頁完成 ({} 張圖片)", page, count));
                    }
                    Err(e) => {
                        eprintln!("❌ 第 {} 頁失敗: {}", page, e);
                        progress.add_failed_page(page);
                    }
                }
            }
            
            // 儲存進度
            {
                let progress = progress_mutex.lock().await;
                self.file_manager.lock().await.save_progress(&progress)?;
            }
            
            // 批次間延遲
            if batch_end < self.total_pages {
                tokio::time::sleep(
                    tokio::time::Duration::from_millis(self.config.batch_delay_ms)
                ).await;
            }
        }
        
        main_pb.finish_with_message("✨ 所有頁面爬取完成！");
        image_pb.finish();
        status_pb.finish_and_clear();
        
        // 顯示統計
        let final_progress = progress_mutex.lock().await;
        println!("\n╔══════════════════════════════════╗");
        println!("║       📊 爬取統計               ║");
        println!("╠══════════════════════════════════╣");
        println!("║ 總頁數:   {:>20} ║", self.total_pages);
        println!("║ 已完成:   {:>20} ║", final_progress.last_completed_page);
        println!("║ 圖片總數: {:>20} ║", final_progress.total_images_downloaded);
        println!("║ 失敗頁面: {:>20} ║", final_progress.failed_pages.len());
        if !final_progress.failed_pages.is_empty() {
            println!("║ 失敗清單: {:?}", final_progress.failed_pages);
        }
        println!("╚══════════════════════════════════╝");
        
        Ok(())
    }
    
    /// 靜態方法：下載並儲存圖片
    async fn download_and_save_image_static(
        file_manager: &Arc<Mutex<FileManager>>,
        url: &str,
        name: &str,
        page: u32
    ) -> Result<()> {
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());
        
        let ext = url.rsplit('.').next().unwrap_or("jpg");
        let filename = format!("{}_{}.{}", &hash[..8], sanitize_filename(name), ext);
        
        let metadata = ImageMetadata {
            filename: filename.clone(),
            description: name.to_string(),
            url: url.to_string(),
            content_hash: hash,
            page_number: page,
            downloaded_at: Utc::now(),
        };
        
        let fm = file_manager.lock().await;
        fm.save_image(&filename, &bytes)?;
        fm.append_metadata(&metadata)?;
        
        Ok(())
    }
}

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