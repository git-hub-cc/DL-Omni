use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::SeekFrom;
use tokio::io::{AsyncWriteExt, AsyncSeekExt};
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use tauri::AppHandle;
use std::str::FromStr;
use crate::models::{MediaInfo, Task, TaskStatus};
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

/// 针对直链或分片流的原生多线程下载引擎
pub async fn download_native(_app: AppHandle, state: AppState, task: &Task) -> Result<u64, String> {
    let mut headers = HeaderMap::new();
    if let Some(headers_json) = &task.http_headers {
        if let Ok(parsed_headers) = serde_json::from_str::<std::collections::HashMap<String, String>>(headers_json) {
            for (k, v) in parsed_headers {
                let clean_v = v.replace('\n', "").replace('\r', "");
                // 使用 from_bytes 包容 Cookie 里的非标字符，防止转换崩溃
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

    // 发起 HEAD 请求获取元数据
    if let Ok(res) = client.head(&task.url).send().await {
        total_size = res.content_length().unwrap_or(0);
        
        // 尝试从 Content-Disposition 提取真实文件名
        if let Some(cd) = res.headers().get(reqwest::header::CONTENT_DISPOSITION).and_then(|v| v.to_str().ok()) {
            real_filename = utils::parse_filename_from_header(cd);
        }
        // 尝试从 Content-Type 提取真实后缀
        if let Some(ct) = res.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
            if let Some(ext) = utils::get_extension_from_mime(ct) {
                real_ext = Some(ext.to_string());
            }
        }
    }

    // 如果 HEAD 不支持，使用 GET byte=0-0 降级获取
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

    let (save_dir, mut threads) = {
        let config = state.config.lock().await;
        (
            config.settings.default_download_path.clone(),
            config.settings.max_threads_per_task as u64
        )
    };

    if is_stream_fallback || total_size < 1024 * 1024 * 5 {
        threads = 1; 
    }

    std::fs::create_dir_all(&save_dir).map_err(|e| e.to_string())?;

    // ================= 文件名终极修复逻辑 =================
    let mut base_name = task.title.clone();
    if base_name.is_empty() || base_name == "unknown_file" || base_name.starts_with("嗅探资源") {
        base_name = utils::extract_filename_from_url(&task.url);
    }
    
    // 清洗由于前端模板造成的畸形 unknown 后缀
    base_name = base_name.replace(".unknown.unknown", "").replace(".unknown", "");
    // 清洗因为模板中 "[title] - [name]" 名字为空而遗留的尾部 " - "
    let trimmed_name = base_name.trim_end_matches(" - ").trim_end_matches(" -").to_string();
    base_name = if trimmed_name.is_empty() { "download_file".to_string() } else { trimmed_name };

    let mut final_filename = base_name.clone();

    // 如果服务器返回了真实名字，融合服务器后缀与前端标题
    if let Some(rf) = real_filename {
        let sanitized_rf = utils::sanitize_filename(&rf);
        if let Some(ext_idx) = sanitized_rf.rfind('.') {
            let ext = &sanitized_rf[ext_idx..]; // 包含 . 点号
            final_filename = format!("{}{}", base_name, ext);
        } else {
            if !final_filename.contains('.') {
                let e = real_ext.unwrap_or_else(|| "mp4".to_string());
                final_filename = format!("{}.{}", final_filename, e);
            }
        }
    } else {
        // 如果服务器没有返回真实名字，使用 MIME 兜底后缀
        if !final_filename.contains('.') {
            let ext = real_ext.unwrap_or_else(|| "mp4".to_string());
            final_filename = format!("{}.{}", final_filename, ext);
        }
    }
    // ======================================================

    let file_path = std::path::Path::new(&save_dir).join(&final_filename);

    {
        let file = std::fs::File::create(&file_path).map_err(|e| e.to_string())?;
        if !is_stream_fallback {
            file.set_len(total_size).map_err(|e| e.to_string())?;
        }
    }

    let downloaded = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];
    let (tx, mut rx) = tokio::sync::mpsc::channel::<(u64, bytes::Bytes)>(threads as usize * 4);

    let writer_path = file_path.clone();
    let writer_handle = tokio::spawn(async move {
        let mut file = tokio::fs::OpenOptions::new().write(true).open(&writer_path).await.unwrap();
        while let Some((offset, chunk)) = rx.recv().await {
            if file.seek(SeekFrom::Start(offset)).await.is_ok() {
                let _ = file.write_all(&chunk).await;
            }
        }
    });

    let reporter_total = total_size;
    let state_clone = state.clone();
    let task_id = task.id.clone();
    let downloaded_clone = downloaded.clone();
    
    let reporter_handle = tokio::spawn(async move {
        let mut last_bytes = 0;
        let mut smoothed_speed = 0.0;
        
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let current_bytes = downloaded_clone.load(Ordering::Relaxed);
            
            let instant_speed = (current_bytes.saturating_sub(last_bytes)) as f64 * 2.0;
            // 使用指数移动平均(EMA)平滑下载速度，避免 UI 数值剧烈跳动
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

    if is_stream_fallback {
        let url = task.url.clone();
        let tx = tx.clone();
        let downloaded = downloaded.clone();
        let client = client.clone();

        handles.push(tokio::spawn(async move {
            if let Ok(mut res) = client.get(&url).send().await {
                let mut current_offset = 0;
                while let Ok(Some(chunk)) = res.chunk().await {
                    let len = chunk.len() as u64;
                    if tx.send((current_offset, chunk)).await.is_err() { break; }
                    current_offset += len;
                    downloaded.fetch_add(len, Ordering::Relaxed);
                }
            }
        }));
    } else {
        let chunk_size = total_size / threads;
        for i in 0..threads {
            let start = i * chunk_size;
            let end = if i == threads - 1 { total_size - 1 } else { (i + 1) * chunk_size - 1 };
            
            let url = task.url.clone();
            let tx = tx.clone();
            let downloaded = downloaded.clone();
            let client = client.clone();

            handles.push(tokio::spawn(async move {
                if let Ok(mut res) = client.get(&url).header("Range", format!("bytes={}-{}", start, end)).send().await {
                    let mut current_offset = start;
                    while let Ok(Some(chunk)) = res.chunk().await {
                        let len = chunk.len() as u64;
                        if tx.send((current_offset, chunk)).await.is_err() { break; }
                        current_offset += len;
                        downloaded.fetch_add(len, Ordering::Relaxed);
                    }
                }
            }));
        }
    }

    drop(tx);

    for handle in handles { let _ = handle.await; }
    let _ = writer_handle.await;
    reporter_handle.abort();

    let final_size = if is_stream_fallback { downloaded.load(Ordering::Relaxed) } else { total_size };

    // 清理失败任务的残留文件
    if final_size == 0 || (total_size > 0 && final_size < total_size) {
        let _ = std::fs::remove_file(&file_path);
        return Err("下载失败: 链接已失效、服务器断开连接或任务被取消".into());
    }

    Ok(final_size)
}