/**
 * 任务的生命周期状态
 */
export type TaskStatus = 
  | 'pending'       // 解析/等待中
  | 'downloading'   // 下载中
  | 'paused'        // 已暂停
  | 'merging'       // 合并中 (如音频+视频)
  | 'error'         // 错误
  | 'completed';    // 已完成

/**
 * 核心下载任务数据结构
 */
export interface Task {
  id: string;                // 唯一 UUID
  url: string;               // 原始媒体链接
  title: string;             // 视频/文件标题
  thumbnail?: string;        // 缩略图路径 (本地缓存或 URL)
  status: TaskStatus;        // 当前状态
  format_id: string;         // 用户选择的 yt-dlp format_id
  total_bytes: number;       // 文件总大小 (字节)
  downloaded_bytes: number;  // 已下载大小 (字节)
  speed: number;             // 当前下载速度 (Bytes/s)
  eta: number;               // 预估剩余时间 (秒)
  created_at: number;        // 任务创建时间戳
  error_msg?: string;        // 错误状态下的简明提示
}

/**
 * 全局用户配置
 */
export interface Config {
  default_download_path: string;
  max_concurrent_tasks: number;
  max_threads_per_task: number;
  proxy_url: string;
  theme: 'dark' | 'light' | 'system';
  yt_dlp_version?: string;
  split_audio_video: boolean;  // 是否分开下载音频与视频（各保存为独立文件）
  video_quality: string;       // 视频画质偏好: 'best' | '1080p' | '720p' | '480p' | '360p'
  audio_quality: string;       // 音频音质偏好: 'best' | '128k' | '64k'
}

/**
 * yt-dlp 解析返回的单条媒体格式
 */
export interface MediaFormat {
  format_id: string;
  ext: string;               // 扩展名 (如 mp4, webm)
  resolution: string;        // 分辨率 (如 1920x1080)
  filesize?: number;         // 预估文件大小
  vcodec: string;            // 视频编码
  acodec: string;            // 音频编码
  format_note?: string;      // 格式备注 (如 1080p Premium)
}

/**
 * yt-dlp -J 解析返回的媒体元数据
 */
export interface MediaInfo {
  id: string;
  title: string;
  duration: number;
  thumbnail: string;
  formats: MediaFormat[];
}