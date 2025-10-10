use scraper::{Html, Selector};
use anyhow::Result;

/// Parser Trait - 不同網站實作不同的 Parser
pub trait PageParser: Send + Sync {
    /// 解析單頁的圖片列表
    /// 回傳：Vec<(image_url, image_name)>
    fn parse_page(&self, html: &str) -> Result<Vec<(String, String)>>;
    
    /// 取得網站的 base URL（用於處理相對路徑）
    fn base_url(&self) -> &str;
}

/// Memes.tw 的 Parser 實作
pub struct MemesTwParser {
    base_url: String,
    container_selector: Selector,
    name_selector: Selector,
    image_selector: Selector,
}

impl MemesTwParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            base_url: "https://memes.tw".to_string(),
            container_selector: Selector::parse("div.-shadow.mt-3.mx-2.relative")
                .map_err(|e| anyhow::anyhow!("選擇器解析失敗: {:?}", e))?,
            name_selector: Selector::parse("header > b")
                .map_err(|e| anyhow::anyhow!("選擇器解析失敗: {:?}", e))?,
            image_selector: Selector::parse("a > img")
                .map_err(|e| anyhow::anyhow!("選擇器解析失敗: {:?}", e))?,
        })
    }
}

impl PageParser for MemesTwParser {
    fn parse_page(&self, html: &str) -> Result<Vec<(String, String)>> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();
        
        for container in document.select(&self.container_selector) {
            // 提取圖片名稱
            let name = container
                .select(&self.name_selector)
                .next()
                .and_then(|elem| elem.text().next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            
            // 提取圖片 URL
            let image_url = container
                .select(&self.image_selector)
                .next()
                .and_then(|elem| elem.value().attr("src"))
                .map(|s| s.to_string());
            
            if let Some(url) = image_url {
                let full_url = normalize_url(&url, &self.base_url);
                results.push((full_url, name));
            }
        }
        
        Ok(results)
    }
    
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// 通用的 CSS Selector Parser（可配置）
pub struct GenericParser {
    base_url: String,
    config: ParserConfig,
}

/// Parser 配置
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// 容器選擇器（包含單個項目的元素）
    pub container_selector: String,
    /// 圖片 URL 選擇器（相對於容器）
    pub image_selector: String,
    /// 圖片 URL 的屬性名稱（通常是 "src"）
    pub image_attr: String,
    /// 名稱選擇器（相對於容器）
    pub name_selector: String,
    /// 名稱提取方式
    pub name_extraction: NameExtraction,
}

#[derive(Debug, Clone)]
pub enum NameExtraction {
    /// 從元素的文字內容提取
    TextContent,
    /// 從元素的屬性提取
    Attribute(String),
}

impl GenericParser {
    pub fn new(base_url: String, config: ParserConfig) -> Self {
        Self { base_url, config }
    }
    
    /// 建立 Memes.tw 的配置
    pub fn memes_tw() -> Result<Self> {
        let config = ParserConfig {
            container_selector: "div.-shadow.mt-3.mx-2.relative".to_string(),
            image_selector: "a > img".to_string(),
            image_attr: "src".to_string(),
            name_selector: "header > b".to_string(),
            name_extraction: NameExtraction::TextContent,
        };
        
        Ok(Self::new("https://memes.tw".to_string(), config))
    }
    
    /// 建立自訂配置（範例：假設的另一個網站）
    #[allow(dead_code)]
    pub fn custom_site(base_url: &str, config: ParserConfig) -> Self {
        Self::new(base_url.to_string(), config)
    }
}

impl PageParser for GenericParser {
    fn parse_page(&self, html: &str) -> Result<Vec<(String, String)>> {
        let document = Html::parse_document(html);
        
        let container_selector = Selector::parse(&self.config.container_selector)
            .map_err(|e| anyhow::anyhow!("容器選擇器錯誤: {:?}", e))?;
        
        let image_selector = Selector::parse(&self.config.image_selector)
            .map_err(|e| anyhow::anyhow!("圖片選擇器錯誤: {:?}", e))?;
        
        let name_selector = Selector::parse(&self.config.name_selector)
            .map_err(|e| anyhow::anyhow!("名稱選擇器錯誤: {:?}", e))?;
        
        let mut results = Vec::new();
        
        for container in document.select(&container_selector) {
            // 提取名稱
            let name = container
                .select(&name_selector)
                .next()
                .map(|elem| match &self.config.name_extraction {
                    NameExtraction::TextContent => {
                        elem.text().next()
                            .unwrap_or("unknown")
                            .trim()
                            .to_string()
                    }
                    NameExtraction::Attribute(attr) => {
                        elem.value()
                            .attr(attr)
                            .unwrap_or("unknown")
                            .to_string()
                    }
                })
                .unwrap_or_else(|| "unknown".to_string());
            
            // 提取圖片 URL
            let image_url = container
                .select(&image_selector)
                .next()
                .and_then(|elem| elem.value().attr(&self.config.image_attr))
                .map(|s| s.to_string());
            
            if let Some(url) = image_url {
                let full_url = normalize_url(&url, &self.base_url);
                results.push((full_url, name));
            }
        }
        
        Ok(results)
    }
    
    fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// 正規化 URL（處理相對路徑）
fn normalize_url(url: &str, base_url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("//") {
        format!("https:{}", url)
    } else if url.starts_with('/') {
        format!("{}{}", base_url, url)
    } else {
        format!("{}/{}", base_url, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memes_tw_parser() {
        let html = r#"
        <div class="row no-gutters mx-n2">
            <div class="-shadow mt-3 mx-2 relative">
                <header><b>測試圖片1</b></header>
                <a><img src="/images/test1.jpg" /></a>
            </div>
        </div>
        "#;
        
        let parser = MemesTwParser::new().unwrap();
        let results = parser.parse_page(html).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "測試圖片1");
        assert!(results[0].0.contains("test1.jpg"));
    }
    
    #[test]
    fn test_generic_parser() {
        let html = r#"
        <div class="item">
            <h2 class="title">圖片標題</h2>
            <img class="photo" data-src="photo.jpg" />
        </div>
        "#;
        
        let config = ParserConfig {
            container_selector: "div.item".to_string(),
            image_selector: "img.photo".to_string(),
            image_attr: "data-src".to_string(),
            name_selector: "h2.title".to_string(),
            name_extraction: NameExtraction::TextContent,
        };
        
        let parser = GenericParser::new("https://example.com".to_string(), config);
        let results = parser.parse_page(html).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "圖片標題");
        assert_eq!(results[0].0, "https://example.com/photo.jpg");
    }
}