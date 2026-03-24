use scraper::{Html, Selector};
use regex::Regex;
use super::types::{FieldRule, ScrapedItem};
use std::collections::HashMap;

/// Extract all items from an HTML document using the given field rules.
/// Each rule produces one value per "row". If a selector matches multiple
/// elements, only the first is used for that field. To scrape a *list*
/// (e.g. multiple products), wrap this in an outer item selector.
pub fn extract_items(html: &str, field_rules: &[FieldRule], item_selector: Option<&str>) -> Vec<ScrapedItem> {
    let document = Html::parse_document(html);

    // If an item selector is given, iterate over each matched element and
    // extract fields within that scope.
    if let Some(item_sel_str) = item_selector {
        if let Ok(item_sel) = Selector::parse(item_sel_str) {
            return document
                .select(&item_sel)
                .map(|el| {
                    let fragment_html = el.html();
                    let fragment = Html::parse_fragment(&fragment_html);
                    extract_single_item(&fragment, field_rules)
                })
                .collect();
        }
    }

    // Otherwise extract one item from the whole page.
    let item = extract_single_item(&document, field_rules);
    if item.fields.is_empty() {
        vec![]
    } else {
        vec![item]
    }
}

/// Extract a single ScrapedItem from an HTML fragment using field rules.
fn extract_single_item(doc: &Html, field_rules: &[FieldRule]) -> ScrapedItem {
    let mut fields = HashMap::new();

    for rule in field_rules {
        let sel = match Selector::parse(&rule.css_selector) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let element = match doc.select(&sel).next() {
            Some(el) => el,
            None => {
                fields.insert(rule.name.clone(), String::new());
                continue;
            }
        };

        let raw_value = match rule.extract.as_str() {
            "text" => element.text().collect::<Vec<_>>().join(" ").trim().to_string(),
            "html" | "inner_html" => element.inner_html(),
            "outer_html" => element.html(),
            attr => element.value().attr(attr).unwrap_or("").to_string(),
        };

        // Apply optional regex filter.
        let value = if let Some(ref pattern) = rule.regex_filter {
            match Regex::new(pattern) {
                Ok(re) => re
                    .captures(&raw_value)
                    .and_then(|c| c.get(1).or(c.get(0)))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or(raw_value),
                Err(_) => raw_value,
            }
        } else {
            raw_value
        };

        fields.insert(rule.name.clone(), value);
    }

    ScrapedItem { fields }
}

/// Extract all links matching a CSS selector from an HTML document.
pub fn extract_links(html: &str, link_selector: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let sel = match Selector::parse(link_selector) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    document
        .select(&sel)
        .filter_map(|el| {
            el.value().attr("href").map(|href| resolve_url(href, base_url))
        })
        .collect()
}

/// Resolve a potentially relative URL against a base URL.
fn resolve_url(href: &str, base_url: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        return format!("https:{}", href);
    }
    if let Ok(base) = url::Url::parse(base_url) {
        if let Ok(resolved) = base.join(href) {
            return resolved.to_string();
        }
    }
    format!("{}/{}", base_url.trim_end_matches('/'), href.trim_start_matches('/'))
}
