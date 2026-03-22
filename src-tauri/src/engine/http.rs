use reqwest::Client;
use std::collections::HashMap;

pub struct VideoClient {
    client: Client,
    pub default_headers: HashMap<String, String>,
}

impl VideoClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let mut default_headers = HashMap::new();
        default_headers.insert("Accept".into(), "*/*".into());
        default_headers.insert("Accept-Language".into(), "en-US,en;q=0.9".into());
        default_headers.insert("Sec-Fetch-Dest".into(), "video".into());
        default_headers.insert("Sec-Fetch-Mode".into(), "no-cors".into());
        default_headers.insert("Sec-Fetch-Site".into(), "cross-site".into());
        Self { client, default_headers }
    }

    pub fn with_referer(mut self, page_url: &str) -> Self {
        self.default_headers.insert("Referer".into(), page_url.into());
        if let Ok(url) = url::Url::parse(page_url) {
            let origin = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
            self.default_headers.insert("Origin".into(), origin);
        }
        self
    }

    pub fn with_cookies(mut self, cookies: &str) -> Self {
        if !cookies.is_empty() {
            self.default_headers.insert("Cookie".into(), cookies.into());
        }
        self
    }

    pub async fn get_text(&self, url: &str) -> Result<String, String> {
        let mut req = self.client.get(url);
        for (k, v) in &self.default_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.text().await.map_err(|e| e.to_string())
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut req = self.client.get(url);
        for (k, v) in &self.default_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string())
    }

    pub async fn head_content_length(&self, url: &str) -> Option<u64> {
        let mut req = self.client.head(url);
        for (k, v) in &self.default_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await.ok()?;
        resp.headers()
            .get(reqwest::header::CONTENT_LENGTH)?
            .to_str().ok()?
            .parse().ok()
    }

    pub fn get_streaming(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(url);
        for (k, v) in &self.default_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req
    }
}
