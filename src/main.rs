mod types;
mod file_manager;
mod fetcher;
mod parser;
mod crawler;
mod dedup;
mod reverse_search;

use crawler::{CrawlerEngine, CrawlerConfig};
use parser::GenericParser;
use dedup::DedupAnalyzer;
use reverse_search::{ReverseSearchEngine, KeywordFilter};
use anyhow::Result;
use std::sync::Arc;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "crawl" => run_crawler().await?,
            "dedup" => run_dedup(args.get(2).map(|s| s.as_str())).await?,
            "search" => run_reverse_search(args.get(2).map(|s| s.as_str())).await?,
            "search-stats" => reverse_search::print_statistics("./data/reverse_search_results.jsonl")?,
            "--help" | "-h" => print_help(),
            _ => {
                println!("未知命令: {}", args[1]);
                print_help();
            }
        }
    } else {
        run_crawler().await?;
    }
    
    Ok(())
}

async fn run_crawler() -> Result<()> {
    println!("=== Memes Crawler ===\n");
    
    let parser = Arc::new(GenericParser::memes_tw()?);
    
    let config = CrawlerConfig::default()
        .with_concurrency(10)
        .with_timeout(30);
    
    let crawler = CrawlerEngine::new(
        "./data",
        "https://memes.tw/maker".to_string(),
        1594,
        parser,
        config,
    )?;
    
    crawler.run().await?;
    
    println!("\n✨ 爬蟲完成！");
    println!("\n💡 下一步：");
    println!("  - cargo run dedup          # 分析重複圖片");
    println!("  - cargo run search         # 反向搜尋");
    
    Ok(())
}

async fn run_dedup(mode: Option<&str>) -> Result<()> {
    println!("=== 重複圖片分析 ===\n");
    
    let analyzer = DedupAnalyzer::new("./data")?;
    let result = analyzer.analyze()?;
    
    result.print_report();
    analyzer.mark_duplicates(&result)?;
    
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
        }
    }
    
    Ok(())
}

async fn run_reverse_search(service_name: Option<&str>) -> Result<()> {
    println!("=== 反向圖片搜尋 ===\n");
    
    let filter = KeywordFilter {
        blocklist: vec![
            "porn".to_string(),
            "xxx".to_string(),
            "adult".to_string(),
            "sex".to_string(),
        ],
        allowlist: vec![],
        min_length: 3,
    };
    
    let mut services: Vec<Arc<dyn reverse_search::ReverseSearchService>> = vec![];
    
    match service_name {
        Some("tineye") => {
            services.push(Arc::new(
                reverse_search::services::tineye::TinEyeService::new()?
            ));
        }
        Some("bing") => {
            services.push(Arc::new(
                reverse_search::services::bing::BingService::new(filter.clone())?
            ));
        }
        Some("all") => {
            services.push(Arc::new(
                reverse_search::services::tineye::TinEyeService::new()?
            ));
            services.push(Arc::new(
                reverse_search::services::bing::BingService::new(filter.clone())?
            ));
        }
        None => {
            // 預設使用 TinEye
            services.push(Arc::new(
                reverse_search::services::tineye::TinEyeService::new()?
            ));
        }
        Some(other) => {
            println!("❌ 未知服務: {}", other);
            println!("可用服務: tineye, bing, all");
            return Ok(());
        }
    }
    
    println!("⚙️  設定：");
    println!("  - 服務: {}", 
        services.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
    );
    println!("  - 並發數: 1");
    println!("  - 關鍵字最小長度: {}", filter.min_length);
    println!("  - 黑名單: {:?}\n", filter.blocklist);
    
    let engine = ReverseSearchEngine::new("./data", services, 1)?;
    
    let progress = engine.load_progress()?;
    if !progress.completed_files.is_empty() {
        println!("📋 已完成 {} 張圖片", progress.completed_files.len());
        println!("⏭️  將從上次中斷處繼續\n");
    }
    
    println!("⚠️  注意：");
    println!("  - 可以隨時 Ctrl+C 中斷，下次會自動繼續");
    println!("  - 進度會自動儲存\n");
    
    println!("確定要開始嗎？(y/N)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if input.trim().to_lowercase() != "y" {
        println!("❌ 已取消");
        return Ok(());
    }
    
    engine.run().await?;
    
    println!("\n💡 查看結果：");
    println!("  - cargo run search-stats");
    
    Ok(())
}

fn print_help() {
    println!("Memes Crawler - 圖片爬蟲工具\n");
    println!("用法:");
    println!("  cargo run                        # 執行爬蟲");
    println!("  cargo run crawl                  # 執行爬蟲");
    println!("  cargo run dedup [preview|remove] # 分析/刪除重複圖片");
    println!("  cargo run search [service]       # 反向圖片搜尋");
    println!("  cargo run search-stats           # 顯示搜尋統計");
    println!("  cargo run --help                 # 顯示此幫助\n");
    println!("反向搜尋服務:");
    println!("  tineye   - TinEye 反向搜尋 (預設)");
    println!("  bing     - Bing 反向搜尋");
    println!("  all      - 使用所有服務\n");
    println!("範例:");
    println!("  cargo run search tineye          # 只用 TinEye");
    println!("  cargo run search bing            # 只用 Bing");
    println!("  cargo run search all             # 兩個都用\n");
    println!("資料檔案:");
    println!("  ./data/images/                      # 圖片");
    println!("  ./data/metadata.jsonl               # 圖片 metadata");
    println!("  ./data/progress.json                # 爬蟲進度");
    println!("  ./data/duplicates.json              # 重複圖片");
    println!("  ./data/search_progress.json         # 搜尋進度");
    println!("  ./data/reverse_search_results.jsonl # 搜尋結果");
}