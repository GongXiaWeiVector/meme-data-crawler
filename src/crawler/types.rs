/// 爬蟲配置
#[derive(Debug, Clone)]
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

impl CrawlerConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }
    
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}