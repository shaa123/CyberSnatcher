use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single scraped data item (row).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedItem {
    pub fields: HashMap<String, String>,
}

/// Defines a field to extract from the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRule {
    /// Name of the field (becomes column header in exports).
    pub name: String,
    /// CSS selector to locate the element.
    pub css_selector: String,
    /// What to extract: "text", "html", or an attribute name like "href", "src".
    pub extract: String,
    /// Optional regex to apply after extraction.
    pub regex_filter: Option<String>,
}

/// A crawl rule that tells the spider how to follow links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlRule {
    /// CSS selector for links to follow.
    pub link_selector: String,
    /// Optional URL pattern (regex) that links must match.
    pub url_pattern: Option<String>,
}

/// Configuration for a spider — analogous to a Scrapy Spider class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiderConfig {
    /// Human-readable name for this spider.
    pub name: String,
    /// Starting URLs to crawl.
    pub start_urls: Vec<String>,
    /// Fields to extract on each page.
    pub field_rules: Vec<FieldRule>,
    /// Optional crawl rules for following links (pagination, etc.).
    pub crawl_rules: Vec<CrawlRule>,
    /// Max pages to crawl (0 = unlimited, default 50).
    pub max_pages: usize,
    /// Max concurrent requests.
    pub concurrency: usize,
    /// Delay between requests in milliseconds.
    pub request_delay_ms: u64,
    /// Respect robots.txt.
    pub respect_robots: bool,
    /// Custom User-Agent string.
    pub user_agent: Option<String>,
    /// Custom headers.
    pub headers: HashMap<String, String>,
}

impl Default for SpiderConfig {
    fn default() -> Self {
        Self {
            name: "default".into(),
            start_urls: vec![],
            field_rules: vec![],
            crawl_rules: vec![],
            max_pages: 50,
            concurrency: 4,
            request_delay_ms: 1000,
            respect_robots: true,
            user_agent: None,
            headers: HashMap::new(),
        }
    }
}

/// Real-time progress event emitted during a scrape job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeProgress {
    pub job_id: String,
    pub pages_crawled: usize,
    pub pages_total: usize,
    pub items_scraped: usize,
    pub current_url: String,
    pub status: String,
    pub log_line: String,
}

/// The final result returned when a scrape job completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub job_id: String,
    pub items: Vec<ScrapedItem>,
    pub pages_crawled: usize,
    pub errors: Vec<String>,
    pub export_path: Option<String>,
}

/// Export format for scraped data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
    JsonLines,
}
