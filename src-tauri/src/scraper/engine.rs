use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use regex::Regex;

use super::middleware;
use super::pipeline;
use super::selector;
use super::spider;
use super::types::*;

/// Run a scrape job and return results. Sends progress updates via the channel.
pub async fn run_spider(
    job_id: String,
    config: SpiderConfig,
    progress_tx: mpsc::UnboundedSender<ScrapeProgress>,
) -> ScrapeResult {
    // Validate config.
    if let Err(e) = spider::validate_config(&config) {
        let _ = progress_tx.send(ScrapeProgress {
            job_id: job_id.clone(),
            pages_crawled: 0,
            pages_total: 0,
            items_scraped: 0,
            current_url: String::new(),
            status: "error".into(),
            log_line: format!("Config error: {}", e),
        });
        return ScrapeResult {
            job_id,
            items: vec![],
            pages_crawled: 0,
            errors: vec![e],
            export_path: None,
        };
    }

    // Build HTTP client.
    let client = match middleware::build_client(
        config.user_agent.as_deref(),
        &config.headers,
    ) {
        Ok(c) => c,
        Err(e) => {
            return ScrapeResult {
                job_id,
                items: vec![],
                pages_crawled: 0,
                errors: vec![e],
                export_path: None,
            };
        }
    };

    let max_pages = if config.max_pages == 0 { usize::MAX } else { config.max_pages };
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let delay = std::time::Duration::from_millis(config.request_delay_ms);

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = config.start_urls.clone();
    let mut all_items: Vec<ScrapedItem> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut pages_crawled: usize = 0;

    // Compile URL pattern regexes for crawl rules.
    let crawl_patterns: Vec<(String, Option<Regex>)> = config
        .crawl_rules
        .iter()
        .map(|rule| {
            let re = rule.url_pattern.as_ref().and_then(|p| Regex::new(p).ok());
            (rule.link_selector.clone(), re)
        })
        .collect();

    while let Some(url) = queue.pop() {
        if pages_crawled >= max_pages {
            break;
        }
        if visited.contains(&url) {
            continue;
        }
        visited.insert(url.clone());

        // Respect robots.txt if configured.
        if config.respect_robots {
            if !middleware::is_allowed_by_robots(&client, &url).await {
                let _ = progress_tx.send(ScrapeProgress {
                    job_id: job_id.clone(),
                    pages_crawled,
                    pages_total: pages_crawled + queue.len(),
                    items_scraped: all_items.len(),
                    current_url: url.clone(),
                    status: "skipped".into(),
                    log_line: format!("Blocked by robots.txt: {}", url),
                });
                continue;
            }
        }

        // Acquire concurrency permit.
        let _permit = semaphore.acquire().await.unwrap();

        let _ = progress_tx.send(ScrapeProgress {
            job_id: job_id.clone(),
            pages_crawled,
            pages_total: pages_crawled + queue.len() + 1,
            items_scraped: all_items.len(),
            current_url: url.clone(),
            status: "crawling".into(),
            log_line: format!("Fetching: {}", url),
        });

        // Fetch the page.
        let html = match client.get(&url).send().await {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let err = format!("HTTP {} for {}", resp.status(), url);
                    errors.push(err.clone());
                    let _ = progress_tx.send(ScrapeProgress {
                        job_id: job_id.clone(),
                        pages_crawled,
                        pages_total: pages_crawled + queue.len(),
                        items_scraped: all_items.len(),
                        current_url: url.clone(),
                        status: "error".into(),
                        log_line: err,
                    });
                    continue;
                }
                match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        errors.push(format!("Failed to read body from {}: {}", url, e));
                        continue;
                    }
                }
            }
            Err(e) => {
                let err = format!("Request failed for {}: {}", url, e);
                errors.push(err.clone());
                let _ = progress_tx.send(ScrapeProgress {
                    job_id: job_id.clone(),
                    pages_crawled,
                    pages_total: pages_crawled + queue.len(),
                    items_scraped: all_items.len(),
                    current_url: url.clone(),
                    status: "error".into(),
                    log_line: err,
                });
                continue;
            }
        };

        pages_crawled += 1;

        // Extract items from the page.
        let items = selector::extract_items(&html, &config.field_rules, None);
        all_items.extend(items);

        let _ = progress_tx.send(ScrapeProgress {
            job_id: job_id.clone(),
            pages_crawled,
            pages_total: pages_crawled + queue.len(),
            items_scraped: all_items.len(),
            current_url: url.clone(),
            status: "scraped".into(),
            log_line: format!(
                "Scraped {} (total items: {})",
                url, all_items.len()
            ),
        });

        // Follow links based on crawl rules.
        for (link_sel, url_pattern) in &crawl_patterns {
            let links = selector::extract_links(&html, link_sel, &url);
            for link in links {
                if visited.contains(&link) {
                    continue;
                }
                // Apply URL pattern filter.
                if let Some(ref re) = url_pattern {
                    if !re.is_match(&link) {
                        continue;
                    }
                }
                queue.push(link);
            }
        }

        // Polite delay between requests.
        if !queue.is_empty() {
            tokio::time::sleep(delay).await;
        }
    }

    // Run the item pipeline (clean, dedup, drop empty).
    pipeline::run_pipeline(&mut all_items);

    let _ = progress_tx.send(ScrapeProgress {
        job_id: job_id.clone(),
        pages_crawled,
        pages_total: pages_crawled,
        items_scraped: all_items.len(),
        current_url: String::new(),
        status: "complete".into(),
        log_line: format!(
            "Done! Crawled {} pages, scraped {} items, {} errors",
            pages_crawled,
            all_items.len(),
            errors.len()
        ),
    });

    ScrapeResult {
        job_id,
        items: all_items,
        pages_crawled,
        errors,
        export_path: None,
    }
}
