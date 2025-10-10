use crate::types::ImageMetadata;
use super::types::ReverseSearchResult;
use anyhow::Result;

/// 反向搜尋服務 Trait
#[async_trait::async_trait]
pub trait ReverseSearchService: Send + Sync {
    /// 服務名稱
    fn name(&self) -> &str;
    
    /// 搜尋單張圖片
    async fn search(&self, metadata: &ImageMetadata) -> Result<ReverseSearchResult>;
    
    /// 是否需要 API key
    fn requires_api_key(&self) -> bool {
        false
    }
    
    /// 建議的延遲時間（毫秒）
    fn suggested_delay_ms(&self) -> u64 {
        1000
    }
}