use obfstr::obfstr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// ── Gumroad API response types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GumroadVerifyResponse {
    success: bool,
    purchase: Option<GumroadPurchase>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GumroadPurchase {
    seller_id: Option<String>,
    product_id: Option<String>,
    product_name: Option<String>,
    email: Option<String>,
    license_key: Option<String>,
    refunded: Option<bool>,
    disputed: Option<bool>,
    chargebacked: Option<bool>,
}

// ── Public types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub activated: bool,
    pub license_key: Option<String>,
    pub email: Option<String>,
    pub product_name: Option<String>,
}

impl Default for LicenseStatus {
    fn default() -> Self {
        Self {
            activated: false,
            license_key: None,
            email: None,
            product_name: None,
        }
    }
}

/// Persisted license data saved to disk.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedLicense {
    license_key: String,
    email: Option<String>,
    product_name: Option<String>,
}

// ── Managed state ───────────────────────────────────────────────────────────

pub struct LicenseState {
    pub status: Mutex<LicenseStatus>,
}

impl LicenseState {
    pub fn new() -> Self {
        let status = load_persisted_license().unwrap_or_default();
        Self {
            status: Mutex::new(status),
        }
    }

    pub fn is_activated(&self) -> bool {
        self.status.lock().unwrap().activated
    }
}

// ── Persistence ─────────────────────────────────────────────────────────────

fn license_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(obfstr!("CyberSnatcher")).join(obfstr!("license.json")))
}

fn load_persisted_license() -> Option<LicenseStatus> {
    let path = license_file_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    let persisted: PersistedLicense = serde_json::from_str(&data).ok()?;

    Some(LicenseStatus {
        activated: true,
        license_key: Some(persisted.license_key),
        email: persisted.email,
        product_name: persisted.product_name,
    })
}

fn save_license(status: &LicenseStatus) -> Result<(), String> {
    let key = status.license_key.as_ref().ok_or_else(|| obfstr!("No license key to save").to_string())?;
    let path = license_file_path().ok_or_else(|| obfstr!("Cannot resolve config directory").to_string())?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("{}{}", obfstr!("Failed to create config dir: "), e))?;
    }

    let persisted = PersistedLicense {
        license_key: key.clone(),
        email: status.email.clone(),
        product_name: status.product_name.clone(),
    };

    let json = serde_json::to_string_pretty(&persisted)
        .map_err(|e| format!("{}{}", obfstr!("Failed to serialize license: "), e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("{}{}", obfstr!("Failed to write license file: "), e))?;

    Ok(())
}

fn remove_license_file() {
    if let Some(path) = license_file_path() {
        let _ = std::fs::remove_file(path);
    }
}

// ── Gumroad API verification ────────────────────────────────────────────────

fn verify_with_gumroad(product_id: &str, license_key: &str) -> Result<GumroadVerifyResponse, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(obfstr!("https://api.gumroad.com/v2/licenses/verify"))
        .form(&[
            (obfstr!("product_id"), product_id),
            (obfstr!("license_key"), license_key),
            (obfstr!("increment_uses_count"), "true"),
        ])
        .send()
        .map_err(|e| format!("{}{}", obfstr!("Failed to contact Gumroad: "), e))?;

    if !resp.status().is_success() {
        return Err(format!("{}{}", obfstr!("Gumroad API returned status: "), resp.status()));
    }

    resp.json::<GumroadVerifyResponse>()
        .map_err(|e| format!("{}{}", obfstr!("Failed to parse Gumroad response: "), e))
}

// ── Tauri commands ──────────────────────────────────────────────────────────

/// Verify a Gumroad license key. Pass your Gumroad product permalink/ID.
#[tauri::command]
pub async fn activate_license(
    state: tauri::State<'_, LicenseState>,
    license_key: String,
) -> Result<LicenseStatus, String> {
    let key = license_key.trim().to_string();
    if key.is_empty() {
        return Err(obfstr!("License key cannot be empty").to_string());
    }

    // TODO: Replace with your actual Gumroad product ID
    let product_id = obfstr!("YOUR_GUMROAD_PRODUCT_ID").to_string();

    let result = tokio::task::spawn_blocking(move || {
        verify_with_gumroad(&product_id, &key)
    })
    .await
    .map_err(|e| format!("{}{}", obfstr!("Task failed: "), e))??;

    if !result.success {
        let msg = result.message.unwrap_or_else(|| obfstr!("Invalid license key").to_string());
        return Err(msg);
    }

    let purchase = result.purchase.ok_or_else(|| obfstr!("No purchase data returned").to_string())?;

    // Reject refunded / disputed / chargebacked licenses
    if purchase.refunded.unwrap_or(false) {
        return Err(obfstr!("This license has been refunded").to_string());
    }
    if purchase.disputed.unwrap_or(false) || purchase.chargebacked.unwrap_or(false) {
        return Err(obfstr!("This license has been disputed or chargebacked").to_string());
    }

    let new_status = LicenseStatus {
        activated: true,
        license_key: purchase.license_key,
        email: purchase.email,
        product_name: purchase.product_name,
    };

    save_license(&new_status)?;

    let mut current = state.status.lock().unwrap();
    *current = new_status.clone();

    Ok(new_status)
}

/// Deactivate the current license and remove stored data.
#[tauri::command]
pub async fn deactivate_license(
    state: tauri::State<'_, LicenseState>,
) -> Result<(), String> {
    remove_license_file();

    let mut current = state.status.lock().unwrap();
    *current = LicenseStatus::default();

    Ok(())
}

/// Get the current license status without contacting Gumroad.
#[tauri::command]
pub async fn get_license_status(
    state: tauri::State<'_, LicenseState>,
) -> Result<LicenseStatus, String> {
    let current = state.status.lock().unwrap();
    Ok(current.clone())
}

// ── Helpers for gating features ─────────────────────────────────────────────

/// Free-tier quality ceiling. Anything above this height requires a license.
const FREE_MAX_HEIGHT: u32 = 720;

/// Returns an error if the requested quality exceeds free tier limits.
pub fn require_license_for_quality(license: &LicenseState, format_quality: &str) -> Result<(), String> {
    if license.is_activated() {
        return Ok(());
    }

    // "audio" and "best" handled separately — "best" may resolve to >720p
    // but we let yt-dlp decide; explicit high-res selections are gated.
    let quality = format_quality.trim().to_lowercase();
    if quality == "audio" || quality.is_empty() {
        return Ok(());
    }

    if let Some(height_str) = quality.strip_suffix('p') {
        if let Ok(height) = height_str.parse::<u32>() {
            if height > FREE_MAX_HEIGHT {
                return Err(format!(
                    "{}{}{}",
                    obfstr!("Quality "),
                    format_quality,
                    obfstr!(" requires a license. Free tier supports up to 720p.")
                ));
            }
        }
    }

    // "best" without explicit height — allow (may or may not exceed 720p)
    Ok(())
}

/// Returns an error if conversion is attempted without a license.
pub fn require_license_for_conversion(license: &LicenseState) -> Result<(), String> {
    if license.is_activated() {
        return Ok(());
    }
    Err(obfstr!("Video conversion requires a license. Please activate your Gumroad key to unlock this feature.").to_string())
}
