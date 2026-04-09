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
  playlist_items?: string;   // 合集下载范围 (如 "1,3,5-7")
  
  // 【重要】绑定的自定义 HTTP 请求头 (JSON 字符串格式)。
  http_headers?: string;     

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
  split_audio_video: boolean;  
  video_quality: string;       
  audio_quality: string;       
  use_cookie: boolean;         // 修改：是否使用内置浏览器的 Cookie
  include_metadata: boolean;
  naming_template: string;     // 新增：标题命名模板配置
  sniff_blacklist: string;     // 新增：嗅探黑名单正则表达式
}

/**
 * yt-dlp 解析返回的单条媒体格式
 */
export interface MediaFormat {
  format_id: string;
  ext: string;               
  resolution: string;        
  filesize?: number;         
  vcodec: string;            
  acodec: string;            
  format_note?: string;      
}

/**
 * 合集子项简要信息
 */
export interface PlaylistItem {
  playlist_index?: number;
  title: string;
  duration?: number;
  url?: string;
  id?: string;
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
  playlist_entries?: PlaylistItem[]; 
}

/**
 * 猫抓级嗅探器捕获的资源数据结构
 */
export interface SniffedResource {
  url: string;
  type: string;             // 如 "video", "media (octet-stream)" 等
  filename: string;         // Fallback 旧字段
  page_title?: string;      // 网页抓取的 Meta Title
  original_name?: string;   // URL 分割推断名
  ext?: string;             // 扩展名后缀
  
  // 动态提取的请求头集合
  headers?: Record<string, string>; 
}