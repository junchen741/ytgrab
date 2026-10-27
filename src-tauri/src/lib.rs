pub mod commands;
pub mod errors;
pub mod ffmpeg_args;
pub mod filename;
pub mod logging;
pub mod media;
pub mod peaks;
pub mod report;
pub mod segments;
pub mod settings;
pub mod sponsorblock;
pub mod state;
pub mod title;
pub mod tools;
pub mod video_url;
pub mod ytdlp;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let paths = app.path();
            logging::init(paths.app_log_dir()?);
            logging::info(format!("YTGrab {} 啟動", app.package_info().version));

            let resource_bin = paths.resource_dir()?.join("bin");
            // 不加 `?`：yt-dlp 複製失敗也要讓程式開起來，等到要用時再顯示說明
            let tools = tools::Tools::prepare(&resource_bin, &paths.app_local_data_dir()?.join("bin"));
            let cache_dir = paths.app_cache_dir()?;
            let _ = std::fs::remove_dir_all(cache_dir.join("sessions"));
            let settings_path = paths.app_config_dir()?.join("settings.json");
            let settings = settings::load(&settings_path, paths.audio_dir()?.join("YouTube 音訊"));
            app.manage(state::AppState::new(tools, resource_bin, cache_dir, settings_path, settings));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<state::AppState>();
                commands::update_ytdlp(&handle, state.inner()).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::fetch_info,
            commands::prepare_audio,
            commands::export_mp3,
            commands::set_output_dir,
            commands::copy_error_report,
            commands::open_special,
            commands::clipboard_youtube_url,
        ])
        .run(tauri::generate_context!())
        .expect("啟動 YTGrab 失敗");
}
