mod commands;
pub mod dash;
pub mod engine;
pub mod ffmpeg;
pub mod hls;
pub mod mp4patch;
mod types;
mod ytdlp;

use commands::browser::BrowserState;
use types::DownloadManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(DownloadManager::new())
        .manage(BrowserState::new())
        .invoke_handler(tauri::generate_handler![
            // Existing commands
            commands::analyze::analyze_url,
            commands::analyze::check_ytdlp,
            commands::analyze::get_ytdlp_version,
            commands::analyze::update_ytdlp,
            commands::download::start_download,
            commands::download::cancel_download,
            commands::convert::convert_file,
            commands::convert::check_ffmpeg,
            commands::convert::get_media_info,
            commands::native::native_download,
            commands::settings::get_settings,
            commands::settings::set_download_folder,
            commands::settings::create_folder,
            commands::settings::delete_folder,
            commands::settings::list_folder_contents,
            commands::settings::open_in_explorer,
            commands::settings::open_file,
            commands::settings::show_in_folder,
            // Browser commands
            commands::browser::open_browser_view,
            commands::browser::navigate_browser,
            commands::browser::resize_browser,
            commands::browser::close_browser,
            commands::browser::browser_go_back,
            commands::browser::browser_go_forward,
            commands::browser::browser_refresh,
            commands::browser::get_detected_videos,
            commands::browser::show_browser,
            commands::browser::hide_browser,
            commands::browser::get_browser_cookies,
            commands::browser::remove_detected_video,
            commands::browser::get_browser_settings,
            commands::browser::set_browser_settings,
            // Stream commands
            commands::stream::parse_hls,
            commands::stream::download_hls_stream,
            commands::stream::download_hls_live_stream,
            commands::stream::parse_dash,
            commands::stream::download_dash_stream,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CyberSnatcher");
}
