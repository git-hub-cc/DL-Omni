use tauri::{AppHandle, State, command};
use crate::state::AppState;
use crate::models::{MediaInfo, Task, Config};
use crate::engine;

#[command]
pub async fn parse_url(
    url: String,
    app: AppHandle,
    state: State<'_, AppState>
) -> Result<MediaInfo, String> {
    if crate::utils::is_direct_link(&url) {
        return engine::downloader::get_direct_link_info(&url).await;
    }

    engine::ytdlp::parse_media_info(&url, app, state.inner().clone())
        .await
        .map_err(|e| format!("解析失败: {}", e))
}

#[command]
pub async fn create_task(
    url: String,
    title: String,
    thumbnail: Option<String>,
    format_id: String,
    playlist_items: Option<String>, 
    http_headers: Option<String>, // 【新增】接收前端传来的动态 Header (序列化后的 JSON 字符串)
    app: AppHandle,
    state: State<'_, AppState>
) -> Result<String, String> {
    let task_id = uuid::Uuid::new_v4().to_string();

    let new_task = Task::new(
        task_id.clone(),
        url.clone(),
        title,
        thumbnail,
        format_id.clone(),
        playlist_items,
        http_headers // 【新增】将 Header 存入任务结构体
    );

    {
        let db = state.db.lock().await;
        db.insert_task(&new_task).map_err(|e| e.to_string())?;
    }

    engine::dispatch_task(app, state.inner().clone(), new_task).await?;
    Ok(task_id)
}

#[command]
pub async fn pause_task(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut active_tasks = state.active_tasks.lock().await;
    if let Some(handle) = active_tasks.remove(&task_id) {
        handle.abort();
    }

    let db = state.db.lock().await;
    db.update_status(&task_id, crate::models::TaskStatus::Paused).map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub async fn resume_task(task_id: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let task = {
        let db = state.db.lock().await;
        db.get_task(&task_id).map_err(|e| e.to_string())?
    };

    if let Some(t) = task {
        engine::dispatch_task(app, state.inner().clone(), t).await?;
        Ok(())
    } else {
        Err("Task not found".into())
    }
}

#[command]
pub async fn get_all_tasks(state: State<'_, AppState>) -> Result<Vec<Task>, String> {
    let db = state.db.lock().await;
    db.get_all_tasks().map_err(|e| e.to_string())
}

#[command]
pub async fn cancel_task(task_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut active_tasks = state.active_tasks.lock().await;
    if let Some(handle) = active_tasks.remove(&task_id) {
        handle.abort();
    }

    let db = state.db.lock().await;
    db.delete_task(&task_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().await;
    db.clear_history().map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn open_folder(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().await;
    let output_dir = &config.settings.default_download_path;

    if std::path::Path::new(output_dir).exists() {
        std::process::Command::new("explorer")
            .arg(output_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub async fn check_engine_update(app: AppHandle) -> Result<String, String> {
    engine::updater::check_and_update(app).await.map_err(|e| e.to_string())
}

#[command]
pub async fn update_config(new_config: Config, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.update(new_config).map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().await;
    Ok(config.settings.clone())
}

#[command]
pub async fn start_sniffing(url: String, app: AppHandle) -> Result<(), String> {
    engine::sniffer::init_sniffer(url, app).await.map_err(|e| format!("启动嗅探器失败: {}", e))
}

#[command]
pub async fn stop_sniffing(app: AppHandle) -> Result<(), String> {
    engine::sniffer::stop_sniffer(app).await.map_err(|e| format!("停止嗅探器失败: {}", e))
}