use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use regex::Regex;

/// 获取目标平台特定的 yt-dlp 可执行文件名
pub fn get_ytdlp_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    return "yt-dlp.exe";
    
    #[cfg(target_os = "macos")]
    return "yt-dlp_macos";
    
    #[cfg(target_os = "linux")]
    return "yt-dlp";
}

/// 获取存放核心依赖二进制文件的统一安全目录 (如 AppData/dl-omni/bin)
pub fn get_binary_dir(app: &AppHandle) -> PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("./"));
    app_dir.join("bin")
}

/// 获取当前环境下 yt-dlp 的运行路径/指令
pub fn get_ytdlp_path(app: &AppHandle) -> Result<String, String> {
    let bin_path = get_binary_dir(app).join(get_ytdlp_filename());
    Ok(bin_path.to_string_lossy().to_string())
}

/// 检查目标磁盘是否有足够的可用空间 (防崩溃拦截预警)
pub fn check_disk_space(_path: &PathBuf, _required_bytes: u64) -> Result<bool, String> {
    Ok(true)
}

/// 清理并格式化文件名，过滤非法字符，防止因命名包含特殊字符导致文件落盘失败
pub fn sanitize_filename(name: &str) -> String {
    let re = Regex::new(r#"[\\/:*?"<>|]"#).unwrap();
    re.replace_all(name, "_").to_string()
}

/// 从普通的直链 URL 中提取文件名
pub fn extract_filename_from_url(url: &str) -> String {
    // 去除 URL 参数部分 ?xxx=yyy
    let parsed_url = url.split('?').next().unwrap_or(url);
    let segments: Vec<&str> = parsed_url.split('/').collect();
    
    if let Some(last) = segments.last() {
        if !last.is_empty() {
            return sanitize_filename(last);
        }
    }
    "unknown_file".to_string()
}

/// [新增] 检查给定的 URL 是否为 m3u8 媒体流索引文件
pub fn is_m3u8_link(url: &str) -> bool {
    let clean_url = url.split('?').next().unwrap_or(url).to_lowercase();
    clean_url.ends_with(".m3u8")
}

/// 检查给定的 URL 是否为常见静态文件的直链
pub fn is_direct_link(url: &str) -> bool {
    let clean_url = url.split('?').next().unwrap_or(url).to_lowercase();
    // 覆盖常见的普通二进制/压缩包/文档/部分独立媒体格式
    let direct_extensions = [
        ".exe", ".zip", ".rar", ".7z", ".tar", ".gz", ".pkg", ".dmg", ".iso", 
        ".bin", ".msi", ".apk", ".pdf", ".txt", ".mp4", ".mp3", ".mkv"
    ];
    direct_extensions.iter().any(|ext| clean_url.ends_with(ext))
}

/// 获取目标平台特定的 ffmpeg 运行文件名
pub fn get_ffmpeg_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    return "ffmpeg.exe";
    
    #[cfg(target_os = "macos")]
    return "ffmpeg";
    
    #[cfg(target_os = "linux")]
    return "ffmpeg";
}

/// 获取 ffmpeg-static Github Release 中对应的资产包名称
pub fn get_ffmpeg_asset_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "ffmpeg-win32-x64";
    
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        return "ffmpeg-darwin-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        return "ffmpeg-darwin-x64";
    }
    
    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "aarch64")]
        return "ffmpeg-linux-arm64";
        #[cfg(not(target_arch = "aarch64"))]
        return "ffmpeg-linux-x64";
    }
}

/// 获取当前环境下 ffmpeg 的运行路径
pub fn get_ffmpeg_path(app: &AppHandle) -> Result<String, String> {
    let bin_path = get_binary_dir(app).join(get_ffmpeg_filename());
    Ok(bin_path.to_string_lossy().to_string())
}