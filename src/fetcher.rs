use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

/// HTTP Fetcher trait - 抽象介面（為未來擴充預留）
pub trait Fetcher {
    async fn fetch_page(&self, url: &str) -> Result<String>;
}

/// HTTP 實作
pub struct HttpFetcher {
    client: Client,
    timeout: Duration,
    max_retries: u32,
}

impl HttpFetcher {
    /// 建立新的 HTTP Fetcher
    pub fn new(timeout_secs: u64, max_retries: u32) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .context("無法建立 HTTP 客戶端")?;

        Ok(Self {
            client,
            timeout: Duration::from_secs(timeout_secs),
            max_retries,
        })
    }

    /// 帶重試的請求
    async fn fetch_with_retry(&self, url: &str) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // 重試前等待（指數退避）
                let wait_time = Duration::from_secs(2u64.pow(attempt - 1));
                tokio::time::sleep(wait_time).await;
                println!("重試 {} - {}", attempt, url);
            }

            match self.client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(body) => return Ok(body),
                            Err(e) => {
                                last_error = Some(anyhow::anyhow!("讀取回應失敗: {}", e));
                                continue;
                            }
                        }
                    } else {
                        last_error = Some(anyhow::anyhow!(
                            "HTTP 錯誤: {}",
                            response.status()
                        ));
                        continue;
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("請求失敗: {}", e));
                    continue;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("未知錯誤")))
    }
}

impl Fetcher for HttpFetcher {
    async fn fetch_page(&self, url: &str) -> Result<String> {
        self.fetch_with_retry(url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch() {
        let fetcher = HttpFetcher::new(30, 3).unwrap();
        let result = fetcher.fetch_page("https://httpbin.org/html").await;
        assert!(result.is_ok());
    }
}