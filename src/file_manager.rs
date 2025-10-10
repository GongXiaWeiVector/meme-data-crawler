use crate::types::{ImageMetadata, Progress};
use anyhow::{Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

/// 檔案操作管理器
pub struct FileManager {
    /// 專案根目錄
    root_dir: String,
}

impl FileManager {
    /// 建立新的檔案管理器
    pub fn new(root_dir: &str) -> Result<Self> {
        // 建立必要的目錄
        fs::create_dir_all(format!("{}/images", root_dir))
            .context("無法建立 images 目錄")?;
        
        Ok(Self {
            root_dir: root_dir.to_string(),
        })
    }

    /// 讀取進度檔案
    pub fn load_progress(&self) -> Result<Progress> {
        let path = format!("{}/progress.json", self.root_dir);
        
        // 如果檔案不存在，回傳新的進度
        if !Path::new(&path).exists() {
            return Ok(Progress::new());
        }

        // 讀取並解析 JSON
        let content = fs::read_to_string(&path)
            .context("無法讀取 progress.json")?;
        
        let progress: Progress = serde_json::from_str(&content)
            .context("無法解析 progress.json")?;
        
        Ok(progress)
    }

    /// 儲存進度檔案（原子性寫入）
    pub fn save_progress(&self, progress: &Progress) -> Result<()> {
        let path = format!("{}/progress.json", self.root_dir);
        let temp_path = format!("{}.tmp", path);

        // 先寫到暫存檔
        let file = File::create(&temp_path)
            .context("無法建立暫存檔")?;
        
        serde_json::to_writer_pretty(file, progress)
            .context("無法寫入 progress.json")?;

        // 原子性地重新命名（避免寫到一半 crash）
        fs::rename(&temp_path, &path)
            .context("無法更新 progress.json")?;

        Ok(())
    }

    /// Append metadata 到 JSONL 檔案
    pub fn append_metadata(&self, metadata: &ImageMetadata) -> Result<()> {
        let path = format!("{}/metadata.jsonl", self.root_dir);
        
        // 以 append 模式開啟檔案
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .context("無法開啟 metadata.jsonl")?;

        let mut writer = BufWriter::new(file);
        
        // 寫入一行 JSON + 換行
        serde_json::to_writer(&mut writer, metadata)
            .context("無法寫入 metadata")?;
        writeln!(writer).context("無法寫入換行符號")?;
        
        writer.flush().context("無法 flush buffer")?;

        Ok(())
    }

    /// 讀取所有 metadata (從 metadata.jsonl)
    pub fn load_all_metadata(&self) -> Result<Vec<ImageMetadata>> {
        let path = format!("{}/metadata.jsonl", self.root_dir);
        
        // 檢查檔案是否存在
        if !Path::new(&path).exists() {
            return Ok(Vec::new());
        }
        
        // 開啟檔案
        let file = File::open(&path)
            .context("無法開啟 metadata.jsonl")?;
        let reader = BufReader::new(file);
        
        // 建立一個空的 Vec 來收集結果
        let mut metadata_list = Vec::new();
        
        // 逐行讀取並解析
        for line in reader.lines() {
            let line = line.context("讀取行失敗")?;
            
            // 跳過空行
            if line.trim().is_empty() {
                continue;
            }
            
            // 解析 JSON
            let metadata: ImageMetadata = serde_json::from_str(&line)
                .context("解析 metadata 失敗")?;
            
            // 加入到列表中
            metadata_list.push(metadata);
        }
        
        Ok(metadata_list)
    }

    /// 重寫 metadata.jsonl（用於去重後更新）
    pub fn rewrite_metadata(&self, metadata_list: &[ImageMetadata]) -> Result<()> {
        let path = format!("{}/metadata.jsonl", self.root_dir);
        let temp_path = format!("{}.tmp", path);
        
        // 先寫到暫存檔
        let file = File::create(&temp_path)
            .context("無法建立暫存檔")?;
        let mut writer = BufWriter::new(file);
        
        for metadata in metadata_list {
            serde_json::to_writer(&mut writer, metadata)
                .context("無法寫入 metadata")?;
            writeln!(writer).context("無法寫入換行符號")?;
        }
        
        writer.flush().context("無法 flush buffer")?;
        
        // 原子性地重新命名
        fs::rename(&temp_path, &path)
            .context("無法更新 metadata.jsonl")?;
        
        Ok(())
    }

    /// 備份 metadata.jsonl
    pub fn backup_metadata(&self) -> Result<()> {
        let path = format!("{}/metadata.jsonl", self.root_dir);
        let backup_path = format!("{}/metadata.jsonl.backup", self.root_dir);
        
        if Path::new(&path).exists() {
            fs::copy(&path, &backup_path)
                .context("無法備份 metadata.jsonl")?;
            println!("📦 已備份 metadata.jsonl -> metadata.jsonl.backup");
        }
        
        Ok(())
    }

    /// 儲存圖片檔案
    pub fn save_image(&self, filename: &str, data: &[u8]) -> Result<()> {
        let path = format!("{}/images/{}", self.root_dir, filename);
        fs::write(&path, data)
            .context("無法寫入圖片檔案")?;
        Ok(())
    }

    /// 取得圖片儲存路徑
    pub fn get_image_path(&self, filename: &str) -> String {
        format!("{}/images/{}", self.root_dir, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_manager() {
        let manager = FileManager::new("./test_data").unwrap();
        
        // 測試進度
        let mut progress = Progress::new();
        progress.update(5, 10);
        manager.save_progress(&progress).unwrap();
        
        let loaded = manager.load_progress().unwrap();
        assert_eq!(loaded.last_completed_page, 5);
        
        // 清理
        std::fs::remove_dir_all("./test_data").ok();
    }
}