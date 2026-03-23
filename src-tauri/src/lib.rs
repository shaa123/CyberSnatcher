mod commands;
pub mod engine;
pub mod ffmpeg;
pub mod mp4patch;
mod types;
mod ytdlp;

use types::DownloadManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(DownloadManager::new())
        .invoke_handler(tauri::generate_handler![
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running CyberSnatcher");
}
