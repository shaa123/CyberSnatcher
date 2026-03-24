use super::types::ScrapedItem;

/// Strip leading/trailing whitespace and collapse internal whitespace in all fields.
pub fn clean_whitespace(items: &mut [ScrapedItem]) {
    for item in items.iter_mut() {
        for value in item.fields.values_mut() {
            let cleaned: String = value
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            *value = cleaned;
        }
    }
}

/// Remove items that have all empty fields.
pub fn drop_empty(items: &mut Vec<ScrapedItem>) {
    items.retain(|item| item.fields.values().any(|v| !v.trim().is_empty()));
}

/// Deduplicate items based on all field values.
pub fn deduplicate(items: &mut Vec<ScrapedItem>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|item| {
        let mut fields_sorted: Vec<_> = item.fields.iter().collect();
        fields_sorted.sort_by_key(|(k, _)| (*k).clone());
        let key: String = fields_sorted
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("|");
        seen.insert(key)
    });
}

/// Run the full default pipeline on a set of items.
pub fn run_pipeline(items: &mut Vec<ScrapedItem>) {
    clean_whitespace(items);
    drop_empty(items);
    deduplicate(items);
}
