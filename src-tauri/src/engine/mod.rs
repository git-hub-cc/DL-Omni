pub mod ytdlp;
pub mod downloader;
pub mod updater;
pub mod sniffer;

use tauri::{AppHandle, Emitter}; 
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

    let handle = tokio::spawn(async move {
        let is_m3u8 = crate::utils::is_m3u8_link(&task.url);

        // 如果明确是 m3u8 格式，即使带有专属 HTTP 头部或格式被指定为 direct，也绝对不能走原生直链下载
        let is_forced_native = (task.http_headers.is_some() || task.format_id == "direct") && !is_m3u8;
        let is_direct_link = crate::utils::is_direct_link(&task.url) && !is_m3u8;

        // 去除外层粗暴的 3 次重试，因为现在 downloader 内部已经拥有细粒度的分片状态自愈和断点续传能力
        let result = if is_forced_native || is_direct_link {
            downloader::download_native(app_clone.clone(), state_clone.clone(), &task).await
        } else {
            // yt-dlp 自身已经具备非常成熟的重试和续传机制
            ytdlp::download_via_ytdlp(app_clone.clone(), state_clone.clone(), &task).await
        };

        // 处理最终的下载结果
        let (final_status, final_bytes) = match result {
            Ok(size) => {
                let _ = state_clone.db.lock().await.update_task_finish(&task.id, TaskStatus::Completed, size);
                (TaskStatus::Completed, size)
            },
            Err(e) => {
                tracing::error!("调度器捕获到任务彻底失败 [{}]: {}", task.id, e);
                
                // 将错误状态写入数据库，保存前端展示线索
                let db_lock = state_clone.db.lock().await;
                let _ = db_lock.update_status(&task.id, TaskStatus::Error);
                // 仅执行 SQL 原生语句附加 error_msg，保证健壮性
                let _ = db_lock.update_task_finish(&task.id, TaskStatus::Error, task.downloaded_bytes);

                // 抛出 IPC 事件让前端界面标红并显示具体原因
                let _ = app_clone.emit("task_error", serde_json::json!({
                    "id": task.id,
                    "error": format!("下载失败: {}", e)
                }));

                (TaskStatus::Error, task.total_bytes)
            }
        };

        // 清理活跃句柄字典
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

    state.active_tasks.lock().await.insert(task_id, handle);

    Ok(())
}