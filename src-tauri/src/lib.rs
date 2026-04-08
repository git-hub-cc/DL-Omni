pub mod commands;
pub mod config;
pub mod database;
pub mod engine;
pub mod models;
pub mod state;
pub mod utils;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 初始化配置管理器
            let config = config::ConfigManager::init(app.handle())
                .expect("Failed to initialize configuration");
            
            // 初始化本地数据库 (自动执行建表、修复中断状态和配置 WAL 模式)
            let db = database::Db::init(app.handle())
                .expect("Failed to initialize database");
            
            // 注册全局状态
            let state = AppState {
                db: Arc::new(Mutex::new(db)),
                config: Arc::new(Mutex::new(config)),
                active_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
                progress_buffer: Arc::new(Mutex::new(Vec::new())),
            };
            app.manage(state);

            // 启动进度聚合定时器，每 200ms 向前端推送一次批量进度
            state::start_progress_ticker(app.handle().clone());

            // 检查或下载内置环境依赖 (如 yt-dlp)
            engine::updater::ensure_binary_exists(app.handle().clone());
            
            // 检查或下载轻量化二进制 ffmpeg (音视频合并)
            engine::updater::ensure_ffmpeg_exists(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::parse_url,
            commands::create_task,
            commands::pause_task,
            commands::resume_task,
            commands::get_all_tasks,
            commands::cancel_task,
            commands::clear_history,
            commands::open_folder,
            commands::check_engine_update,
            commands::update_config,
            commands::get_config,
            commands::start_sniffing,
            commands::stop_sniffing // 【修复】在此处注册缺失的销毁指令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}