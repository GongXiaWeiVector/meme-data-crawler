mod types;
mod file_manager;
mod fetcher;
mod parser;
mod crawler;

use crawler::{Crawler, CrawlerConfig};
use parser::{GenericParser, ParserConfig, NameExtraction};
use anyhow::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Memes Crawler ===\n");
    
    // 方式 1：使用內建的 Memes.tw parser
    let parser = Arc::new(GenericParser::memes_tw()?);
    
    /* 方式 2：自訂配置（適用於其他網站）
    let parser_config = ParserConfig {
        container_selector: "div.item".to_string(),
        image_selector: "img.photo".to_string(),
        image_attr: "src".to_string(),
        name_selector: "h2.title".to_string(),
        name_extraction: NameExtraction::TextContent,
    };
    let parser = Arc::new(GenericParser::new(
        "https://example.com".to_string(),
        parser_config
    ));
    */
    
    // 爬蟲配置
    let config = CrawlerConfig {
        concurrency: 10,        // 同時 10 個請求
        timeout_secs: 30,
        max_retries: 3,
        batch_delay_ms: 1000,
    };
    
    // 建立爬蟲
    let crawler = Crawler::new(
        "./data",                           // 資料目錄
        "https://memes.tw/maker".to_string(), // Base URL
        1594,                               // 總頁數
        parser,                             // 可插拔的 parser
        config
    )?;
    
    // 執行
    crawler.run().await?;
    
    println!("\n爬蟲完成！");
    println!("資料儲存在 ./data 目錄");
    println!("- ./data/images/        # 圖片檔案");
    println!("- ./data/metadata.jsonl # 圖片 metadata");
    println!("- ./data/progress.json  # 爬取進度");
    
    Ok(())
}