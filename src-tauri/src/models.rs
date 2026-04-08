use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Downloading,
    Paused,
    Merging,
    Error,
    Completed,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub status: TaskStatus,
    pub format_id: String,
    pub playlist_items: Option<String>, 
    pub http_headers: Option<String>, // 【新增】保存嗅探到的专属请求头 (如 JSON 格式的 Referer/User-Agent)
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub speed: f64,
    pub eta: u64,
    pub created_at: i64,
    pub error_msg: Option<String>,
}

impl Task {
    pub fn new(
        id: String,
        url: String,
        title: String,
        thumbnail: Option<String>,
        format_id: String,
        playlist_items: Option<String>,
        http_headers: Option<String>, // 【新增】支持传入动态 Header
    ) -> Self {
        Self {
            id,
            url,
            title,
            thumbnail,
            status: TaskStatus::Pending,
            format_id,
            playlist_items,
            http_headers,
            total_bytes: 0,
            downloaded_bytes: 0,
            speed: 0.0,
            eta: 0,
            created_at: chrono::Utc::now().timestamp_millis(),
            error_msg: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaFormat {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub filesize: Option<u64>,
    pub vcodec: String,
    pub acodec: String,
    pub format_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistItem {
    pub playlist_index: Option<u32>,
    pub title: String,
    pub duration: Option<f64>,
    pub url: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaInfo {
    pub id: String,
    pub title: String,
    pub duration: f64,
    pub thumbnail: String,
    pub formats: Vec<MediaFormat>,
    pub playlist_entries: Option<Vec<PlaylistItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_download_path: String,
    pub max_concurrent_tasks: u8,
    pub max_threads_per_task: u8,
    pub proxy_url: Option<String>,
    pub theme: String,
    pub yt_dlp_version: Option<String>,
    pub split_audio_video: bool,
    pub video_quality: String,
    pub audio_quality: String,
    pub browser_cookie: Option<String>,
    pub include_metadata: bool,
}

// 【新增】专门用于接收前端嗅探器发送的复杂资源数据结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SniffedResource {
    pub url: String,
    pub r#type: String,
    pub filename: String,
    pub headers: Option<std::collections::HashMap<String, String>>, // 动态请求头集合
}