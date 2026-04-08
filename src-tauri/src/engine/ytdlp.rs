use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tauri::AppHandle;
use regex::Regex;
use crate::models::{MediaInfo, PlaylistItem, Task, TaskStatus};
use crate::state::{AppState, TaskProgressUpdate};
use crate::utils;

/// 调用 yt-dlp -J 解析链接的元数据
pub async fn parse_media_info(url: &str, app: AppHandle, state: AppState) -> Result<MediaInfo, String> {
    let ytdlp_path = utils::get_ytdlp_path(&app)?;

    let browser_cookie = {
        let config = state.config.lock().await;
        config.settings.browser_cookie.clone()
    };

    let mut cmd = Command::new(&ytdlp_path);

    cmd.arg("--dump-single-json") 
        .arg("--flat-playlist")
        .arg("--no-warnings")      
        .arg(url);

    // 挂载：使用本地浏览器 Cookie 绕过限制
    if let Some(cookie) = browser_cookie {
        if cookie != "none" && !cookie.is_empty() {
            cmd.arg("--cookies-from-browser").arg(cookie);
        }
    }

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let output = cmd.output()
        .await
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        // 友好的异常拦截：提示 Cookie 数据库被锁定
        if err.contains("database is locked") || err.contains("Could not copy Chrome cookie database") {
            return Err("浏览器Cookie数据库正被占用，请彻底关闭该浏览器后再尝试解析！".into());
        }
        return Err(format!("yt-dlp error: {}", err));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    // 提取合集中的子项列表
    let mut playlist_entries = None;
    if let Some(entries) = v.get("entries").and_then(|e| e.as_array()) {
        let mut items = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            items.push(PlaylistItem {
                playlist_index: entry.get("playlist_index").and_then(|idx| idx.as_u64()).map(|idx| idx as u32).or(Some((i + 1) as u32)),
                title: entry.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown").to_string(),
                duration: entry.get("duration").and_then(|d| d.as_f64()),
                url: entry.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()),
                id: entry.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()),
            });
        }
        playlist_entries = Some(items);
    }

    Ok(MediaInfo {
        id: v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
        title: v.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown Title").to_string(),
        duration: v.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0),
        thumbnail: v.get("thumbnail").and_then(|t| t.as_str()).unwrap_or("").to_string(),
        formats: vec![], // 这里暂时略过 formats 的深度解析，前端按规则过滤即可
        playlist_entries,
    })
}

// 辅助函数保持原样
fn parse_size_to_bytes(val: f64, unit: &str) -> u64 {
    let multiplier = if unit.contains("GiB") || unit.contains("G") { 1024.0 * 1024.0 * 1024.0 }
    else if unit.contains("MiB") || unit.contains("M") { 1024.0 * 1024.0 }
    else if unit.contains("KiB") || unit.contains("K") { 1024.0 }
    else { 1.0 };
    (val * multiplier) as u64
}

fn parse_eta(eta_str: &str) -> u64 {
    let parts: Vec<&str> = eta_str.split(':').collect();
    let mut seconds = 0;
    let mut multiplier = 1;
    for part in parts.iter().rev() {
        if let Ok(val) = part.parse::<u64>() {
            seconds += val * multiplier;
            multiplier *= 60;
        }
    }
    seconds
}

/// 核心下载逻辑
pub async fn download_via_ytdlp(app: AppHandle, state: AppState, task: &Task) -> Result<u64, String> {
    let ytdlp_path = utils::get_ytdlp_path(&app)?;

    let (save_dir, max_threads, split_av, browser_cookie, include_metadata) = {
        let config = state.config.lock().await;
        (
            config.settings.default_download_path.clone(),
            config.settings.max_threads_per_task.max(1),
            config.settings.split_audio_video,
            config.settings.browser_cookie.clone(),
            config.settings.include_metadata
        )
    };

    let mut format_arg = task.format_id.clone();

    // 合并与分开下载配置
    if split_av {
        if format_arg.contains('+') { format_arg = format_arg.replace('+', ","); }
        else if !format_arg.contains(',') && format_arg != "best" { format_arg = format!("{},bestaudio", format_arg); }
    } else {
        if format_arg.contains(',') { format_arg = format_arg.replace(',', "+"); }
        else if !format_arg.contains('+') && format_arg != "best" { format_arg = format!("{}+bestaudio", format_arg); }
    }

    let mut cmd = Command::new(&ytdlp_path);
    cmd.arg("-f").arg(&format_arg);

    // FFmpeg 挂载
    if !split_av {
        cmd.arg("--merge-output-format").arg("mp4");
        if let Ok(ffmpeg_path) = utils::get_ffmpeg_path(&app) {
            cmd.arg("--ffmpeg-location").arg(ffmpeg_path);
        }
    }

    // --- 新特性 1: 挂载浏览器 Cookie ---
    if let Some(cookie) = browser_cookie {
        if cookie != "none" && !cookie.is_empty() {
            cmd.arg("--cookies-from-browser").arg(cookie);
        }
    }

    // --- 新特性 2: 下载范围 (合集) ---
    if let Some(ref items) = task.playlist_items {
        if !items.is_empty() {
            cmd.arg("--yes-playlist");
            cmd.arg("--playlist-items").arg(items);
        } else {
            cmd.arg("--no-playlist");
        }
    } else {
        cmd.arg("--no-playlist");
    }

    // --- 新特性 3: 元数据目录模式 ---
    if include_metadata {
        // 创建以视频标题命名的子文件夹
        cmd.arg("-o").arg(format!("{}/%(title)s/%(title)s.%(ext)s", save_dir));
        // 抓取辅助信息
        cmd.arg("--write-thumbnail")
            .arg("--write-info-json")
            .arg("--write-description")
            .arg("--write-subs").arg("--write-auto-subs")
            // 将部分基础属性内嵌进视频文件
            .arg("--embed-metadata")
            .arg("--embed-thumbnail");
    } else {
        cmd.arg("-P").arg(save_dir);
    }

    // --- 【修改】新特性 4: 挂载嗅探器捕获的防盗链 Headers ---
    if let Some(ref headers_json) = task.http_headers {
        if let Ok(parsed_headers) = serde_json::from_str::<std::collections::HashMap<String, String>>(headers_json) {
            for (key, value) in parsed_headers {
                cmd.arg("--add-header").arg(format!("{}: {}", key, value));
            }
        }
    }

    // 基础配置
    cmd.arg("--concurrent-fragments").arg(max_threads.to_string())
        .arg("--newline")
        .arg(&task.url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout).lines();

    // 正则解析器 (兼容多视频队列输出)
    let re_pct = Regex::new(r"\[download\]\s+(?P<pct>[0-9\.]+)%").unwrap();
    let re_frag = Regex::new(r"Frag\s+(?P<cur>\d+)/(?P<tot>\d+)").unwrap();
    let re_size = Regex::new(r"of\s+[~]?(?P<size>[0-9\.]+)(?P<unit>[a-zA-Z]+)").unwrap();
    let re_speed = Regex::new(r"at\s+(?P<speed>[0-9\.]+)(?P<unit>[a-zA-Z/]+)").unwrap();
    let re_eta = Regex::new(r"ETA\s+(?P<eta>[0-9:]+)").unwrap();
    let re_dest = Regex::new(r"\[download\] Destination:\s+(?P<path>.+)").unwrap();
    let re_merge = Regex::new(r#"\[Merger\] Merging formats into "(?P<path>[^"]+)""#).unwrap();
    let re_already = Regex::new(r"\[download\]\s+(?P<path>.*) has already been downloaded").unwrap();

    let mut current_total_bytes = task.total_bytes;
    let mut current_speed = 0.0;
    let mut current_eta = 0;
    let mut final_path: Option<String> = None;

    while let Some(line) = reader.next_line().await.unwrap_or(None) {
        if let Some(caps) = re_merge.captures(&line) {
            final_path = Some(caps.name("path").unwrap().as_str().to_string());
        } else if let Some(caps) = re_dest.captures(&line) {
            final_path = Some(caps.name("path").unwrap().as_str().to_string());
        } else if let Some(caps) = re_already.captures(&line) {
            final_path = Some(caps.name("path").unwrap().as_str().to_string());
        }

        let mut pct: Option<f64> = None;

        if let Some(caps_pct) = re_pct.captures(&line) {
            pct = caps_pct.name("pct").unwrap().as_str().parse().ok();
        } else if let Some(caps_frag) = re_frag.captures(&line) {
            if let (Ok(cur), Ok(tot)) = (
                caps_frag.name("cur").unwrap().as_str().parse::<f64>(),
                caps_frag.name("tot").unwrap().as_str().parse::<f64>()
            ) {
                if tot > 0.0 { pct = Some((cur / tot) * 100.0); }
            }
        }

        if let Some(p) = pct {
            if let Some(caps_size) = re_size.captures(&line) {
                if let Ok(size_val) = caps_size.name("size").unwrap().as_str().parse::<f64>() {
                    let unit = caps_size.name("unit").unwrap().as_str();
                    current_total_bytes = parse_size_to_bytes(size_val, unit);
                }
            }
            if let Some(caps_speed) = re_speed.captures(&line) {
                if let Ok(speed_val) = caps_speed.name("speed").unwrap().as_str().parse::<f64>() {
                    let unit = caps_speed.name("unit").unwrap().as_str();
                    current_speed = parse_size_to_bytes(speed_val, unit) as f64;
                }
            }
            if let Some(caps_eta) = re_eta.captures(&line) {
                current_eta = parse_eta(caps_eta.name("eta").unwrap().as_str());
            }

            let (dl_bytes, t_bytes) = if current_total_bytes > 0 {
                ((current_total_bytes as f64 * (p / 100.0)) as u64, current_total_bytes)
            } else {
                (p as u64, 0)
            };

            let mut buffer = state.progress_buffer.lock().await;
            buffer.push(TaskProgressUpdate {
                id: task.id.clone(),
                downloaded_bytes: dl_bytes,
                total_bytes: t_bytes,
                speed: current_speed,
                eta: current_eta,
                status: TaskStatus::Downloading,
            });
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;

    // 在子进程结束后判断是否异常退出。
    // 为了容错，有些平台报错但文件依然有效，这里做个简单拦截。
    if !status.success() {
        // 尝试抓取 stderr 以抛出详细异常信息
        return Err("Download process exited with error or database locked.".into());
    }

    if let Some(path) = final_path {
        if let Ok(metadata) = std::fs::metadata(&path) {
            let actual_size = metadata.len();
            if actual_size > 0 { current_total_bytes = actual_size; }
        }
    }

    Ok(current_total_bytes)
}