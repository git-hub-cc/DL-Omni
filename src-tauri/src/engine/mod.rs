pub mod ytdlp;
pub mod downloader;
pub mod updater;
pub mod sniffer;

use tauri::{AppHandle, Emitter}; // 引入 Emitter 以便发送错误事件
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
        // 强制嗅探旁路逻辑：增加对 m3u8 的特判拦截
        let is_m3u8 = crate::utils::is_m3u8_link(&task.url);

        // 如果明确是 m3u8 格式，即使带有专属 HTTP 头部或格式被指定为 direct，也绝对不能走原生直链下载
        let is_forced_native = (task.http_headers.is_some() || task.format_id == "direct") && !is_m3u8;
        let is_direct_link = crate::utils::is_direct_link(&task.url) && !is_m3u8;

        // ==========================================
        // 增加 3 次自动重试兜底逻辑
        // ==========================================
        let max_retries = 3;
        let mut current_attempt = 0;
        let mut final_result = Err(String::new());

        while current_attempt < max_retries {
            current_attempt += 1;

            // 如果是重试，给予 3 秒的冷却时间，防止频繁重试被服务器彻底拉黑
            if current_attempt > 1 {
                eprintln!("[DL-Omni] 任务 {} (尝试 {}/{}) 发生异常，等待 3 秒后进行自动重试...", task.id, current_attempt, max_retries);
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }

            let result = if is_forced_native || is_direct_link {
                downloader::download_native(app_clone.clone(), state_clone.clone(), &task).await
            } else {
                ytdlp::download_via_ytdlp(app_clone.clone(), state_clone.clone(), &task).await
            };

            match result {
                Ok(size) => {
                    final_result = Ok(size);
                    break; // 下载成功，跳出重试循环
                }
                Err(e) => {
                    final_result = Err(e); // 记录错误，继续下一次循环
                }
            }
        }

        // 处理最终的下载结果
        let (final_status, final_bytes) = match final_result {
            Ok(size) => {
                let _ = state_clone.db.lock().await.update_task_finish(&task.id, TaskStatus::Completed, size);
                (TaskStatus::Completed, size)
            },
            Err(e) => {
                eprintln!("Task {} failed after {} attempts: {}", task.id, max_retries, e);
                // 将错误状态写入数据库
                let _ = state_clone.db.lock().await.update_status(&task.id, TaskStatus::Error);

                // 【修复】真正将带有原因的 error 抛给前端，前端 IPC 会接管并标红
                let _ = app_clone.emit("task_error", serde_json::json!({
                    "id": task.id,
                    "error": format!("多次重试后依然失败: {}", e)
                }));

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