use crate::types::{ImageMetadata, DuplicateRecord};
use crate::file_manager::FileManager;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;

/// 去重分析器
pub struct DedupAnalyzer {
    file_manager: FileManager,
}

impl DedupAnalyzer {
    pub fn new(data_dir: &str) -> Result<Self> {
        Ok(Self {
            file_manager: FileManager::new(data_dir)?,
        })
    }
    
    /// 分析重複圖片
    pub fn analyze(&self) -> Result<DedupResult> {
        println!("📖 讀取所有 metadata...");
        let all_metadata = self.file_manager.load_all_metadata()?;
        
        println!("🔍 分析中... (共 {} 張圖片)", all_metadata.len());
        
        // hash -> Vec<ImageMetadata>
        let mut hash_map: HashMap<String, Vec<ImageMetadata>> = HashMap::new();
        
        for metadata in all_metadata {
            hash_map
                .entry(metadata.content_hash.clone())
                .or_insert_with(Vec::new)
                .push(metadata);
        }
        
        // 找出重複的
        let mut duplicates = Vec::new();
        let mut unique_count = 0;
        let mut duplicate_count = 0;
        
        for (hash, items) in hash_map.iter() {
            if items.len() > 1 {
                // 有重複
                duplicate_count += items.len() - 1; // 保留一個，其餘算重複
                
                let record = DuplicateRecord {
                    content_hash: hash.clone(),
                    files: items.iter().map(|m| m.filename.clone()).collect(),
                };
                duplicates.push(record);
            } else {
                unique_count += 1;
            }
        }
        
        Ok(DedupResult {
            total_images: hash_map.values().map(|v| v.len()).sum(),
            unique_images: hash_map.len(),
            duplicate_groups: duplicates.len(),
            duplicate_images: duplicate_count,
            duplicates,
        })
    }
    
    /// 標記重複圖片（寫入檔案）
    pub fn mark_duplicates(&self, result: &DedupResult) -> Result<()> {
        println!("💾 儲存重複圖片報告...");
        
        // 儲存到 duplicates.json
        let json = serde_json::to_string_pretty(&result.duplicates)?;
        fs::write("./data/duplicates.json", json)?;
        
        println!("✅ 報告已儲存到 ./data/duplicates.json");
        
        Ok(())
    }
    
    /// 自動刪除重複圖片（保留第一個）
    pub fn remove_duplicates(&self, result: &DedupResult, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("🔍 預覽模式：不會實際刪除檔案");
        } else {
            println!("⚠️  警告：即將刪除重複圖片！");
        }
        
        let mut removed_count = 0;
        
        for dup_group in &result.duplicates {
            // 保留第一個，刪除其餘
            for (i, filename) in dup_group.files.iter().enumerate() {
                if i == 0 {
                    println!("  ✅ 保留: {}", filename);
                    continue;
                }
                
                let path = self.file_manager.get_image_path(filename);
                
                if dry_run {
                    println!("  🗑️  [預覽] 將刪除: {}", filename);
                } else {
                    match fs::remove_file(&path) {
                        Ok(_) => {
                            println!("  ❌ 已刪除: {}", filename);
                            removed_count += 1;
                        }
                        Err(e) => {
                            eprintln!("  ⚠️  刪除失敗 ({}): {}", filename, e);
                        }
                    }
                }
            }
            println!();
        }
        
        if !dry_run {
            println!("✅ 已刪除 {} 個重複檔案", removed_count);
        }
        
        Ok(())
    }
}

/// 去重結果
#[derive(Debug)]
pub struct DedupResult {
    /// 總圖片數
    pub total_images: usize,
    /// 唯一圖片數
    pub unique_images: usize,
    /// 重複組數
    pub duplicate_groups: usize,
    /// 重複圖片數
    pub duplicate_images: usize,
    /// 重複記錄
    pub duplicates: Vec<DuplicateRecord>,
}

impl DedupResult {
    /// 顯示報告
    pub fn print_report(&self) {
        println!("\n╔══════════════════════════════════╗");
        println!("║     🔍 重複圖片分析報告         ║");
        println!("╠══════════════════════════════════╣");
        println!("║ 總圖片數:   {:>18} ║", self.total_images);
        println!("║ 唯一圖片:   {:>18} ║", self.unique_images);
        println!("║ 重複組數:   {:>18} ║", self.duplicate_groups);
        println!("║ 重複圖片:   {:>18} ║", self.duplicate_images);
        println!("║ 重複率:     {:>17.1}% ║", 
            (self.duplicate_images as f64 / self.total_images as f64) * 100.0);
        println!("╚══════════════════════════════════╝\n");
        
        if self.duplicate_groups > 0 {
            println!("📋 重複組詳情 (前 10 組):\n");
            
            for (i, dup) in self.duplicates.iter().take(10).enumerate() {
                println!("  組 {}: {} 張重複", i + 1, dup.files.len());
                println!("  Hash: {}", &dup.content_hash[..16]);
                for (j, file) in dup.files.iter().enumerate() {
                    let marker = if j == 0 { "✅ 保留" } else { "❌ 重複" };
                    println!("    {} {}", marker, file);
                }
                println!();
            }
            
            if self.duplicates.len() > 10 {
                println!("  ... 還有 {} 組重複", self.duplicates.len() - 10);
            }
        }
    }
}