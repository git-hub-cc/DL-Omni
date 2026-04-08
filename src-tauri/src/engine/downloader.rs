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
    // 1. 解析任务自带的动态 HTTP Headers (由嗅探器捕获的防盗链信息)
    let mut headers = HeaderMap::new();
    if let Some(headers_json) = &task.http_headers {
        if let Ok(parsed_headers) = serde_json::from_str::<std::collections::HashMap<String, String>>(headers_json) {
            for (k, v) in parsed_headers {
                if let (Ok(name), Ok(value)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
                    headers.insert(name, value);
                }
            }
        }
    }

    // 2. 注入 Headers 构建 Client
    let client = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers) // 应用动态请求头
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut total_size = 0;

    if let Ok(res) = client.head(&task.url).send().await {
        total_size = res.content_length().unwrap_or(0);
    }

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

    let filename = if task.title.is_empty() || task.title == "unknown_file" {
        utils::extract_filename_from_url(&task.url)
    } else {
        task.title.clone()
    };
    
    // 确保文件名带有正确后缀，对于动态直链非常重要
    let final_filename = if !filename.contains('.') { format!("{}.mp4", filename) } else { filename };
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
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let current_bytes = downloaded_clone.load(Ordering::Relaxed);
            
            let speed = (current_bytes.saturating_sub(last_bytes)) as f64 * 2.0;
            let mut eta = 0;
            if reporter_total > 0 && speed > 0.0 {
                eta = (reporter_total.saturating_sub(current_bytes) as f64 / speed) as u64;
            }

            let mut buffer = state_clone.progress_buffer.lock().await;
            buffer.push(TaskProgressUpdate {
                id: task_id.clone(),
                downloaded_bytes: current_bytes,
                total_bytes: reporter_total,
                speed,
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

    if final_size == 0 {
        return Err("下载失败: 链接已失效或服务器因缺少 Header/Cookie 拒绝了连接 (403/401)".into());
    }

    Ok(final_size)
}