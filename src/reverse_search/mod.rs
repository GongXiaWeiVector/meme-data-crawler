// 宣告子模組
pub mod types;
pub mod trait_def;
pub mod engine;
pub mod utils;
pub mod services;

// 重新導出常用項目（讓外部可以用 reverse_search::XXX 直接存取）
pub use types::{ReverseSearchResult, SearchProgress, KeywordFilter};
pub use trait_def::ReverseSearchService;
pub use engine::ReverseSearchEngine;

use anyhow::Result;
use std::fs;
use std::path::Path;

/// 讀取所有搜尋結果
pub fn load_all_results(results_file: &str) -> Result<Vec<ReverseSearchResult>> {
    if !Path::new(results_file).exists() {
        return Ok(vec![]);
    }
    
    let content = fs::read_to_string(results_file)?;
    let results: Vec<ReverseSearchResult> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    
    Ok(results)
}

/// 顯示統計報告
pub fn print_statistics(results_file: &str) -> Result<()> {
    let results = load_all_results(results_file)?;
    
    if results.is_empty() {
        println!("⚠️  尚無搜尋結果");
        return Ok(());
    }
    
    println!("\n╔══════════════════════════════════╗");
    println!("║   📊 反向搜尋統計報告           ║");
    println!("╠══════════════════════════════════╣");
    println!("║ 總搜尋數:   {:>18} ║", results.len());
    
    let with_title = results.iter().filter(|r| r.best_guess.is_some()).count();
    println!("║ 找到標題:   {:>18} ║", with_title);
    
    let total_keywords: usize = results.iter().map(|r| r.keywords.len()).sum();
    let avg_keywords = if !results.is_empty() {
        total_keywords as f64 / results.len() as f64
    } else {
        0.0
    };
    println!("║ 平均關鍵字: {:>18.1} ║", avg_keywords);
    
    println!("╚══════════════════════════════════╝\n");
    
    // 按服務統計
    use std::collections::HashMap;
    let mut by_service: HashMap<String, usize> = HashMap::new();
    for result in &results {
        *by_service.entry(result.service.clone()).or_insert(0) += 1;
    }
    
    println!("📊 各服務統計:");
    for (service, count) in by_service {
        println!("  - {}: {} 次", service, count);
    }
    println!();
    
    // 顯示範例
    println!("📋 範例結果 (前 5 個):\n");
    for (i, result) in results.iter().take(5).enumerate() {
        println!("{}. {} [{}]", i + 1, result.filename, result.service);
        if let Some(title) = &result.best_guess {
            println!("   標題: {}", title);
        }
        if !result.keywords.is_empty() {
            println!("   關鍵字: {}", result.keywords.join(", "));
        }
        println!();
    }
    
    Ok(())
}