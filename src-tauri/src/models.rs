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

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Downloading => "downloading",
            TaskStatus::Paused => "paused",
            TaskStatus::Merging => "merging",
            TaskStatus::Error => "error",
            TaskStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => TaskStatus::Pending,
            "downloading" => TaskStatus::Downloading,
            "paused" => TaskStatus::Paused,
            "merging" => TaskStatus::Merging,
            "completed" => TaskStatus::Completed,
            _ => TaskStatus::Error,
        }
    }
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
    pub http_headers: Option<String>, 
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
        http_headers: Option<String>, 
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
    pub use_cookie: bool,
    pub include_metadata: bool,
    pub naming_template: String,
    pub sniff_blacklist: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SniffedResource {
    pub url: String,
    pub r#type: String, 
    pub filename: String,
    pub page_title: Option<String>,
    pub original_name: Option<String>,
    pub ext: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>, 
}

// ================= 新增：断点续传状态持久化模型 =================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChunkState {
    pub id: usize,
    pub start: u64,
    pub end: u64,
    pub current_offset: u64,
    pub is_completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskStateFile {
    pub task_id: String,
    pub total_bytes: u64,
    pub file_name: String,
    pub chunks: Vec<ChunkState>,
}