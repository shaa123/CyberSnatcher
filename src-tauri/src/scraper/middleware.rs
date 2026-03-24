use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT};
use std::collections::HashMap;

const DEFAULT_USER_AGENT: &str =
    "CyberSnatcher/1.0 (Scraper Engine; +https://github.com/shaa123/cybersnatcher)";

/// Build a reqwest::Client configured with the spider's settings.
pub fn build_client(
    user_agent: Option<&str>,
    custom_headers: &HashMap<String, String>,
) -> Result<reqwest::Client, String> {
    let mut headers = HeaderMap::new();

    let ua = user_agent.unwrap_or(DEFAULT_USER_AGENT);
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(ua).unwrap_or_else(|_| HeaderValue::from_static(DEFAULT_USER_AGENT)),
    );

    // Add custom headers.
    for (key, value) in custom_headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            headers.insert(name, val);
        }
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Check robots.txt for a URL (basic implementation).
pub async fn is_allowed_by_robots(client: &reqwest::Client, url: &str) -> bool {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return true,
    };

    let robots_url = format!("{}://{}/robots.txt", parsed.scheme(), parsed.host_str().unwrap_or(""));

    let robots_text = match client.get(&robots_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(text) => text,
            Err(_) => return true,
        },
        Err(_) => return true,
    };

    let path = parsed.path();
    let mut current_agent_applies = false;
    let mut is_disallowed = false;

    for line in robots_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(agent) = line.strip_prefix("User-agent:").or_else(|| line.strip_prefix("user-agent:")) {
            let agent = agent.trim();
            current_agent_applies = agent == "*" || agent.eq_ignore_ascii_case("cybersnatcher");
        } else if current_agent_applies {
            if let Some(disallowed) = line.strip_prefix("Disallow:").or_else(|| line.strip_prefix("disallow:")) {
                let disallowed = disallowed.trim();
                if !disallowed.is_empty() && path.starts_with(disallowed) {
                    is_disallowed = true;
                }
            }
            if let Some(allowed) = line.strip_prefix("Allow:").or_else(|| line.strip_prefix("allow:")) {
                let allowed = allowed.trim();
                if !allowed.is_empty() && path.starts_with(allowed) {
                    is_disallowed = false;
                }
            }
        }
    }

    !is_disallowed
}
