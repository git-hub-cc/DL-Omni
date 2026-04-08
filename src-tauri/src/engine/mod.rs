pub mod ytdlp;
pub mod downloader;
pub mod updater;
pub mod sniffer;

use tauri::AppHandle;
use crate::models::{Task, TaskStatus};
use crate::state::{AppState, TaskProgressUpdate};

/// 核心调度器：支持直链多线程与 yt-dlp 双轨制路由
pub async fn dispatch_task(app: AppHandle, state: AppState, mut task: Task) -> Result<(), String> {
    let task_id = task.id.clone();

    // 标记任务为正在下载并更新数据库
    task.status = TaskStatus::Downloading;
    {
        let db = state.db.lock().await;
        let _ = db.update_status(&task_id, TaskStatus::Downloading);
    }

    let state_clone = state.clone();
    let app_clone = app.clone();

    // 创建一个 Tokio 异步任务
    let handle = tokio::spawn(async move {
        // 【修改】强制嗅探旁路逻辑：增加对 m3u8 的特判拦截
        let is_m3u8 = crate::utils::is_m3u8_link(&task.url);
        
        // 如果明确是 m3u8 格式，即使带有专属 HTTP 头部或格式被指定为 direct，也绝对不能走原生直链下载
        let is_forced_native = (task.http_headers.is_some() || task.format_id == "direct") && !is_m3u8;
        let is_direct_link = crate::utils::is_direct_link(&task.url) && !is_m3u8;

        let result = if is_forced_native || is_direct_link {
            downloader::download_native(app_clone, state_clone.clone(), &task).await
        } else {
            ytdlp::download_via_ytdlp(app_clone, state_clone.clone(), &task).await
        };

        // 处理下载结果
        let (final_status, final_bytes) = match result {
            Ok(size) => {
                let _ = state_clone.db.lock().await.update_task_finish(&task.id, TaskStatus::Completed, size);
                (TaskStatus::Completed, size)
            },
            Err(e) => {
                eprintln!("Task {} failed: {}", task.id, e);
                // 将错误信息推送到前端并更新数据库
                let _ = state_clone.db.lock().await.update_status(&task.id, TaskStatus::Error);
                (TaskStatus::Error, task.total_bytes)
            }
        };

        // 清理 active_tasks 集合
        state_clone.active_tasks.lock().await.remove(&task.id);

        // 触发最终状态的进度聚合推送
        let mut buffer = state_clone.progress_buffer.lock().await;
        buffer.push(TaskProgressUpdate {
            id: task.id.clone(),
            downloaded_bytes: final_bytes,
            total_bytes: final_bytes,
            speed: 0.0,
            eta: 0,
            status: final_status,
        });
    });

    // 将任务句柄存入全局状态，以便随时可以 abort（暂停/取消）
    state.active_tasks.lock().await.insert(task_id, handle);

    Ok(())
}