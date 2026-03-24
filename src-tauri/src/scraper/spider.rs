use super::types::SpiderConfig;
use regex::Regex;

/// Validate a spider configuration before running.
pub fn validate_config(config: &SpiderConfig) -> Result<(), String> {
    if config.start_urls.is_empty() {
        return Err("At least one start URL is required".into());
    }

    for url in &config.start_urls {
        if url::Url::parse(url).is_err() {
            return Err(format!("Invalid start URL: {}", url));
        }
    }

    if config.field_rules.is_empty() {
        return Err("At least one field extraction rule is required".into());
    }

    for rule in &config.field_rules {
        if rule.name.is_empty() {
            return Err("Field rule name cannot be empty".into());
        }
        if rule.css_selector.is_empty() {
            return Err(format!("CSS selector for field '{}' cannot be empty", rule.name));
        }
        // Validate regex if provided.
        if let Some(ref pattern) = rule.regex_filter {
            Regex::new(pattern)
                .map_err(|e| format!("Invalid regex for field '{}': {}", rule.name, e))?;
        }
    }

    for rule in &config.crawl_rules {
        if rule.link_selector.is_empty() {
            return Err("Crawl rule link selector cannot be empty".into());
        }
        if let Some(ref pattern) = rule.url_pattern {
            Regex::new(pattern)
                .map_err(|e| format!("Invalid URL pattern in crawl rule: {}", e))?;
        }
    }

    if config.concurrency == 0 {
        return Err("Concurrency must be at least 1".into());
    }

    Ok(())
}
