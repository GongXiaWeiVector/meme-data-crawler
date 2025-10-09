mod types;
mod file_manager;
mod fetcher;
mod parser;
mod crawler;
mod dedup;

use crawler::{Crawler, CrawlerConfig};
use parser::GenericParser;
use dedup::DedupAnalyzer;
use anyhow::Result;
use std::sync::Arc;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    // 檢查命令
    if args.len() > 1 {
        match args[1].as_str() {
            "dedup" => {
                // 去重模式
                run_dedup(args.get(2).map(|s| s.as_str())).await?;
                return Ok(());
            }
            "crawl" => {
                // 明確指定爬蟲模式
                run_crawler().await?;
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                println!("未知命令: {}", args[1]);
                print_help();
                return Ok(());
            }
        }
    }
    
    // 預設：爬蟲模式
    run_crawler().await?;
    
    Ok(())
}

async fn run_crawler() -> Result<()> {
    println!("=== Memes Crawler ===\n");
    
    // 使用 Memes.tw parser
    let parser = Arc::new(GenericParser::memes_tw()?);
    
    // 爬蟲配置
    let config = CrawlerConfig {
        concurrency: 10,
        timeout_secs: 30,
        max_retries: 3,
        batch_delay_ms: 1000,
    };
    
    // 建立爬蟲
    let crawler = Crawler::new(
        "./data",
        "https://memes.tw/maker".to_string(),
        1594,
        parser,
        config
    )?;
    
    // 執行
    crawler.run().await?;
    
    println!("\n✨ 爬蟲完成！");
    println!("\n💡 提示：執行 'cargo run dedup' 來分析重複圖片");
    
    Ok(())
}

async fn run_dedup(mode: Option<&str>) -> Result<()> {
    println!("=== 重複圖片分析 ===\n");
    
    let analyzer = DedupAnalyzer::new("./data")?;
    
    // 分析
    let result = analyzer.analyze()?;
    
    // 顯示報告
    result.print_report();
    
    // 儲存報告
    analyzer.mark_duplicates(&result)?;
    
    // 根據模式處理
    match mode {
        Some("remove") => {
            println!("⚠️  確定要刪除重複圖片嗎？(y/N)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            
            if input.trim().to_lowercase() == "y" {
                analyzer.remove_duplicates(&result, false)?;
            } else {
                println!("❌ 已取消");
            }
        }
        Some("preview") | None => {
            println!("💡 預覽模式：");
            analyzer.remove_duplicates(&result, true)?;
            println!("\n💡 執行 'cargo run dedup remove' 來實際刪除");
        }
        Some(other) => {
            println!("未知模式: {}", other);
            println!("可用模式: preview, remove");
        }
    }
    
    Ok(())
}

fn print_help() {
    println!("Memes Crawler - 圖片爬蟲工具\n");
    println!("用法:");
    println!("  cargo run                    # 執行爬蟲");
    println!("  cargo run crawl              # 執行爬蟲（明確指定）");
    println!("  cargo run dedup              # 分析重複圖片（預覽）");
    println!("  cargo run dedup preview      # 分析重複圖片（預覽）");
    println!("  cargo run dedup remove       # 分析並刪除重複圖片");
    println!("  cargo run --help             # 顯示此幫助\n");
    println!("資料儲存位置:");
    println!("  ./data/images/        # 圖片檔案");
    println!("  ./data/metadata.jsonl # 圖片 metadata");
    println!("  ./data/progress.json  # 爬取進度");
    println!("  ./data/duplicates.json # 重複圖片報告");
}