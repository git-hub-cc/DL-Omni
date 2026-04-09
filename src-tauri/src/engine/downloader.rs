use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::SeekFrom;
use tokio::io::{AsyncWriteExt, AsyncSeekExt};
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use tauri::AppHandle;
use std::str::FromStr;
use tokio::task::JoinSet;

use crate::models::{MediaInfo, Task, TaskStatus, TaskStateFile, ChunkState};
use crate::state::{AppState, TaskProgressUpdate};
use crate::utils;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub async fn get_direct_link_info(url: &str) -> Result<MediaInfo, String> {
    let client = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let _ = client.get(url).header("Range", "bytes=0-0").send().await;
    let filename = utils::extract_filename_from_url(url);

    Ok(MediaInfo {
        id: "direct_link".to_string(),
        title: filename,
        duration: 0.0,
        thumbnail: "".to_string(),
        formats: vec![],
        playlist_entries: None,
    })
}

/// 具备自愈重试与断点续传能力的原生下载引擎
pub async fn download_native(_app: AppHandle, state: AppState, task: &Task) -> Result<u64, String> {
    tracing::info!("初始化下载任务: [{}] {}", task.id, task.title);

    let mut headers = HeaderMap::new();
    if let Some(headers_json) = &task.http_headers {
        if let Ok(parsed_headers) = serde_json::from_str::<std::collections::HashMap<String, String>>(headers_json) {
            for (k, v) in parsed_headers {
                let clean_v = v.replace('\n', "").replace('\r', "");
                if let (Ok(name), Ok(value)) = (HeaderName::from_str(&k), HeaderValue::from_bytes(clean_v.as_bytes())) {
                    headers.insert(name, value);
                }
            }
        }
    }

    let client = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut total_size = 0;
    let mut real_filename: Option<String> = None;
    let mut real_ext: Option<String> = None;

    // 获取文件元数据
    if let Ok(res) = client.head(&task.url).send().await {
        total_size = res.content_length().unwrap_or(0);
        if let Some(cd) = res.headers().get(reqwest::header::CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
            real_filename = utils::parse_filename_from_header(cd);
        }
        if let Some(ct) = res.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
            if let Some(ext) = utils::get_extension_from_mime(ct) {
                real_ext = Some(ext.to_string());
            }
        }
    }

    // 降级获取
    if total_size == 0 {
        if let Ok(res) = client.get(&task.url).header("Range", "bytes=0-0").send().await {
            if let Some(cr) = res.headers().get(reqwest::header::CONTENT_RANGE) {
                if let Ok(s) = cr.to_str() {
                    if let Some(total) = s.split('/').last() {
                        total_size = total.parse().unwrap_or(0);
                    }
                }
            }
            if total_size == 0 {
                total_size = res.content_length().unwrap_or(0);
            }
            if real_filename.is_none() {
                if let Some(cd) = res.headers().get(reqwest::header::CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
                    real_filename = utils::parse_filename_from_header(cd);
                }
            }
            if real_ext.is_none() {
                if let Some(ct) = res.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
                    if let Some(ext) = utils::get_extension_from_mime(ct) {
                        real_ext = Some(ext.to_string());
                    }
                }
            }
        }
    }

    let is_stream_fallback = total_size == 0;
    let (save_dir, mut max_threads) = {
        let config = state.config.lock().await;
        (
            config.settings.default_download_path.clone(),
            config.settings.max_threads_per_task as u64
        )
    };

    if is_stream_fallback || total_size < 1024 * 1024 * 5 {
        max_threads = 1; 
    }

    tokio::fs::create_dir_all(&save_dir).await.map_err(|e| e.to_string())?;

    // ================= 文件名终极修复逻辑 =================
    let mut base_name = task.title.clone();
    if base_name.is_empty() || base_name == "unknown_file" || base_name.starts_with("嗅探资源") {
        base_name = utils::extract_filename_from_url(&task.url);
    }
    base_name = base_name.replace(".unknown.unknown", "").replace(".unknown", "");
    let trimmed_name = base_name.trim_end_matches(" - ").trim_end_matches(" -").to_string();
    base_name = if trimmed_name.is_empty() { "download_file".to_string() } else { trimmed_name };

    let mut final_filename = base_name.clone();
    if let Some(rf) = real_filename {
        let sanitized_rf = utils::sanitize_filename(&rf);
        if let Some(ext_idx) = sanitized_rf.rfind('.') {
            let ext = &sanitized_rf[ext_idx..];
            final_filename = format!("{}{}", base_name, ext);
        } else if !final_filename.contains('.') {
            let e = real_ext.unwrap_or_else(|| "mp4".to_string());
            final_filename = format!("{}.{}", final_filename, e);
        }
    } else if !final_filename.contains('.') {
        let ext = real_ext.unwrap_or_else(|| "mp4".to_string());
        final_filename = format!("{}.{}", final_filename, ext);
    }

    let file_path = std::path::Path::new(&save_dir).join(&final_filename);
    let part_file_path = file_path.with_extension("omni.part");

    tracing::info!("文件路径分配: {:?}", file_path);

    // ================= 断点续传状态机 =================
    let mut state_file = TaskStateFile {
        task_id: task.id.clone(),
        total_bytes: total_size,
        file_name: final_filename.clone(),
        chunks: vec![],
    };

    let mut initial_downloaded = 0;

    if !is_stream_fallback {
        let mut is_resume_valid = false;
        if part_file_path.exists() && file_path.exists() {
            if let Ok(part_data) = tokio::fs::read_to_string(&part_file_path).await {
                if let Ok(saved_state) = serde_json::from_str::<TaskStateFile>(&part_data) {
                    if saved_state.total_bytes == total_size {
                        tracing::info!("检测到有效的断点续传状态，正在恢复...");
                        state_file = saved_state;
                        is_resume_valid = true;
                        
                        for chunk in &state_file.chunks {
                            initial_downloaded += chunk.current_offset;
                        }
                    } else {
                        tracing::warn!("本地进度快照与服务器文件大小不符，将重新开始下载");
                    }
                }
            }
        }

        if !is_resume_valid {
            tracing::info!("创建新的分片分配，总大小: {} bytes，线程数: {}", total_size, max_threads);
            let file = tokio::fs::File::create(&file_path).await.map_err(|e| e.to_string())?;
            file.set_len(total_size).await.map_err(|e| e.to_string())?;
            
            let chunk_size = total_size / max_threads;
            for i in 0..max_threads {
                let start = i * chunk_size;
                let end = if i == max_threads - 1 { total_size - 1 } else { (i + 1) * chunk_size - 1 };
                state_file.chunks.push(ChunkState {
                    id: i as usize,
                    start,
                    end,
                    current_offset: 0,
                    is_completed: false,
                });
            }
            // 写入初始状态
            let json = serde_json::to_string(&state_file).map_err(|e| e.to_string())?;
            tokio::fs::write(&part_file_path, json).await.map_err(|e| e.to_string())?;
        }
    } else {
        tracing::warn!("目标为流式数据或无法获取大小，降级为单线程追加模式");
        let _ = tokio::fs::File::create(&file_path).await.map_err(|e| e.to_string())?;
    }

    let downloaded = Arc::new(AtomicU64::new(initial_downloaded));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, u64, bytes::Bytes)>(max_threads as usize * 4);
    
    // ================= 独立写入与进度持久化线程 =================
    let writer_path = file_path.clone();
    let part_path_clone = part_file_path.clone();
    let mut writer_state = state_file.clone();
    let writer_downloaded = downloaded.clone();
    let is_stream = is_stream_fallback;

    let writer_handle = tokio::spawn(async move {
        let mut file = tokio::fs::OpenOptions::new().write(true).open(&writer_path).await.unwrap();
        let mut last_save = tokio::time::Instant::now();

        while let Some((chunk_id, offset, chunk_data)) = rx.recv().await {
            if file.seek(SeekFrom::Start(offset)).await.is_ok() {
                if file.write_all(&chunk_data).await.is_ok() {
                    let len = chunk_data.len() as u64;
                    writer_downloaded.fetch_add(len, Ordering::Relaxed);
                    
                    if !is_stream {
                        writer_state.chunks[chunk_id].current_offset += len;
                        if writer_state.chunks[chunk_id].current_offset >= (writer_state.chunks[chunk_id].end - writer_state.chunks[chunk_id].start + 1) {
                            writer_state.chunks[chunk_id].is_completed = true;
                        }

                        // 限流刷盘（每 1.5 秒更新一次 .part 文件）
                        if last_save.elapsed().as_millis() >= 1500 {
                            if let Ok(json) = serde_json::to_string(&writer_state) {
                                let _ = tokio::fs::write(&part_path_clone, json).await;
                            }
                            last_save = tokio::time::Instant::now();
                        }
                    }
                }
            }
        }

        // 最终刷盘确保状态一致
        if !is_stream {
            if let Ok(json) = serde_json::to_string(&writer_state) {
                let _ = tokio::fs::write(&part_path_clone, json).await;
            }
        }
    });

    // ================= 前端进度汇报线程 =================
    let reporter_total = total_size;
    let state_clone = state.clone();
    let task_id = task.id.clone();
    let reporter_downloaded = downloaded.clone();
    
    let reporter_handle = tokio::spawn(async move {
        let mut last_bytes = reporter_downloaded.load(Ordering::Relaxed);
        let mut smoothed_speed = 0.0;
        
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let current_bytes = reporter_downloaded.load(Ordering::Relaxed);
            
            let instant_speed = (current_bytes.saturating_sub(last_bytes)) as f64 * 2.0;
            smoothed_speed = if smoothed_speed == 0.0 { instant_speed } else { smoothed_speed * 0.7 + instant_speed * 0.3 };
            
            let mut eta = 0;
            if reporter_total > 0 && smoothed_speed > 0.0 {
                eta = (reporter_total.saturating_sub(current_bytes) as f64 / smoothed_speed) as u64;
            }

            let mut buffer = state_clone.progress_buffer.lock().await;
            buffer.push(TaskProgressUpdate {
                id: task_id.clone(),
                downloaded_bytes: current_bytes,
                total_bytes: reporter_total,
                speed: smoothed_speed,
                eta,
                status: TaskStatus::Downloading,
            });

            last_bytes = current_bytes;
            if reporter_total > 0 && current_bytes >= reporter_total { break; }
        }
    });

    // ================= JoinSet 并发控制与分片重试引擎 =================
    let mut join_set = JoinSet::new();

    if is_stream_fallback {
        let url = task.url.clone();
        let tx_clone = tx.clone();
        let client_clone = client.clone();

        join_set.spawn(async move {
            let mut current_offset = 0;
            let mut retries = 0;
            
            loop {
                // 流式下载如果断开，只能尝试重新发起请求，由于无法断点，只能覆盖追加（有风险，但流式本来就是 fallback）
                let req = client_clone.get(&url).send().await;
                match req {
                    Ok(mut res) => {
                        while let Ok(Some(chunk)) = res.chunk().await {
                            let len = chunk.len() as u64;
                            if tx_clone.send((0, current_offset, chunk)).await.is_err() { return Err("写入通道已关闭".into()); }
                            current_offset += len;
                            // retries = 0; // 成功读取数据，重置重试次数
                        }
                        break; // 正常结束流
                    }
                    Err(e) => {
                        retries += 1;
                        if retries > 5 { return Err(format!("流式下载多次重试失败: {}", e)); }
                        tracing::warn!("流式下载中断，2秒后尝试重连 (重试 {}/5)", retries);
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
            Ok::<(), String>(())
        });
    } else {
        for chunk in state_file.chunks {
            if chunk.is_completed { continue; }

            let url = task.url.clone();
            let tx_clone = tx.clone();
            let client_clone = client.clone();

            join_set.spawn(async move {
                let mut local_offset = chunk.current_offset;
                let mut retries = 0;

                loop {
                    let start = chunk.start + local_offset;
                    let end = chunk.end;

                    if start > end { break; } 

                    tracing::debug!("分片 [{}] 发起请求 Range: bytes={}-{}", chunk.id, start, end);
                    let req = client_clone.get(&url).header("Range", format!("bytes={}-{}", start, end)).send().await;

                    match req {
                        Ok(mut res) => {
                            if !res.status().is_success() {
                                retries += 1;
                                if retries > 5 { return Err(format!("分片 [{}] HTTP 状态异常: {}", chunk.id, res.status())); }
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                continue;
                            }

                            while let Ok(Some(bytes)) = res.chunk().await {
                                let len = bytes.len() as u64;
                                if tx_clone.send((chunk.id, start + (local_offset - chunk.current_offset), bytes)).await.is_err() {
                                    return Err("分片写入通道已关闭".into());
                                }
                                local_offset += len;
                                retries = 0; 
                            }

                            if local_offset >= (chunk.end - chunk.start + 1) {
                                tracing::info!("分片 [{}] 下载完成", chunk.id);
                                break;
                            } else {
                                // 意外中断，进入下一轮重试循环
                                retries += 1;
                                tracing::warn!("分片 [{}] 意外中断，进入重试阶段...", chunk.id);
                            }
                        }
                        Err(e) => {
                            retries += 1;
                            if retries > 5 { return Err(format!("分片 [{}] 网络彻底失败: {}", chunk.id, e)); }
                            let backoff = 2_u64.pow(retries);
                            tracing::warn!("分片 [{}] 网络错误，{}秒后重试: {}", chunk.id, backoff, e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(backoff)).await;
                        }
                    }
                }
                Ok::<(), String>(())
            });
        }
    }

    drop(tx);

    // ================= 严格异常捕获与完整性校验 =================
    let mut has_fatal_error = false;
    let mut error_message = String::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(worker_result) => {
                if let Err(e) = worker_result {
                    has_fatal_error = true;
                    error_message = e;
                    join_set.abort_all(); // 一个分片彻底死亡，终止所有其他分片
                    break;
                }
            }
            Err(e) => {
                // Panic 捕获
                has_fatal_error = true;
                error_message = format!("线程内部崩溃: {}", e);
                join_set.abort_all();
                break;
            }
        }
    }

    let _ = writer_handle.await;
    reporter_handle.abort();

    if has_fatal_error {
        tracing::error!("任务下载失败被终止: {}", error_message);
        return Err(error_message);
    }

    // 物理磁盘双重校验
    let final_size = downloaded.load(Ordering::Relaxed);
    
    if !is_stream_fallback {
        if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
            if metadata.len() != total_size || final_size != total_size {
                tracing::error!("数据完整性校验失败! 预期: {}, 内存累加: {}, 磁盘实际: {}", total_size, final_size, metadata.len());
                let _ = tokio::fs::remove_file(&file_path).await;
                return Err("数据不完整或文件系统写入失败，已被安全销毁".into());
            }
        } else {
            return Err("无法验证最终文件的完整性".into());
        }

        // 校验完美通过，清理状态文件
        let _ = tokio::fs::remove_file(&part_file_path).await;
        tracing::info!("任务校验通过，下载圆满完成。");
    }

    Ok(final_size)
}