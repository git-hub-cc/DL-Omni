use tauri::{AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use std::fs;

/// 获取应用专用的内置 cookies.txt 路径
fn get_internal_cookie_path(app: &AppHandle) -> std::path::PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("./"));
    app_dir.join("cookies.txt")
}

/// 解析单行 raw cookie 并转换为 Netscape HTTP Cookie File 格式的单行
fn format_cookie_to_netscape(domain: &str, raw_cookie: &str) -> String {
    let mut lines = String::new();
    let parts = raw_cookie.split(';');
    for part in parts {
        let kv = part.trim();
        if kv.is_empty() { continue; }
        
        let mut split = kv.splitn(2, '=');
        let name = split.next().unwrap_or("");
        let value = split.next().unwrap_or("");
        
        if name.is_empty() { continue; }

        let formatted_domain = if domain.starts_with('.') { domain.to_string() } else { format!(".{}", domain) };
        let include_subdomains = "TRUE";
        let path = "/";
        let secure = "FALSE";
        // 赋予一个默认的一年后的过期时间时间戳，防止 yt-dlp 拒绝读取
        let expiry = chrono::Utc::now().timestamp() + 31536000;

        lines.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            formatted_domain, include_subdomains, path, secure, expiry, name, value
        ));
    }
    lines
}

/// 初始化高级嗅探器逻辑 (基于 Tauri 2 Webview 与猫抓级多层级脚本注入)
pub async fn init_sniffer(url: String, app: AppHandle) -> Result<(), String> {
    let label = "sniffer_window";

    if let Some(win) = app.get_webview_window(label) {
        let _ = win.close();
    }

    // 核心拦截脚本：实现了 DOM 劫持、更稳健的启发式匹配、MIME 拦截与安全的 JSON 解析
    let init_script = r#"
        (function() {
            console.log("[DL-Omni] 猫抓级高级嗅探脚本已注入，开启底层多路侦听...");
            
            const emittedUrls = new Set();

            function syncCookieToBackend() {
                if (document.cookie && window.__TAURI_INTERNALS__) {
                    window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                        event: "sniffed_cookie",
                        payload: {
                            domain: window.location.hostname,
                            cookie: document.cookie
                        }
                    }).catch(() => {});
                }
            }

            // 监听 DOM 加载完成及定期上报 Cookie 状态
            window.addEventListener('load', syncCookieToBackend);
            setInterval(syncCookieToBackend, 5000);

            // 提取网页元数据
            function getPageMetadata() {
                try {
                    const ogTitle = document.querySelector('meta[property="og:title"]');
                    let title = ogTitle ? ogTitle.getAttribute('content') : document.title;
                    return title ? title.trim() : '未知网页';
                } catch (e) {
                    return '未知网页';
                }
            }

            // [修改点] 智能切割 URL 提取原始文件名和扩展名
            function extractFileInfo(url) {
                try {
                    const parsed = new URL(url);
                    const path = parsed.pathname;
                    const filename = path.split('/').pop() || '';
                    const extMatch = filename.match(/\.([a-zA-Z0-9]+)$/);
                    return {
                        original_name: filename || 'unknown',
                        ext: extMatch ? extMatch[1].toLowerCase() : 'unknown'
                    };
                } catch(e) {
                    return { original_name: 'unknown', ext: 'unknown' };
                }
            }

            function getAbsoluteUrl(url) {
                try {
                    return new URL(url, window.location.href).href;
                } catch(e) {
                    return url;
                }
            }

            // [修改点] 增加 MIME 解析推断，防止 unknown 后缀泛滥
            function tryEmit(url, type, source, mimeType = null) {
                if (!url || typeof url !== 'string' || !url.startsWith('http')) return;

                try {
                    const absUrl = getAbsoluteUrl(url);

                    if (emittedUrls.has(absUrl)) return;
                    emittedUrls.add(absUrl);

                    let fileInfo = extractFileInfo(absUrl);
                    
                    // 如果存在 MIME 类型，且后缀原本未知，进行智能推断覆盖
                    if (mimeType && fileInfo.ext === 'unknown') {
                        const pureMime = mimeType.split(';')[0].trim().toLowerCase();
                        const mimeToExt = {
                            'video/mp4': 'mp4',
                            'video/x-flv': 'flv',
                            'video/x-matroska': 'mkv',
                            'video/webm': 'webm',
                            'video/quicktime': 'mov',
                            'audio/mpeg': 'mp3',
                            'audio/mp4': 'm4a',
                            'application/x-mpegurl': 'm3u8',
                            'application/vnd.apple.mpegurl': 'm3u8',
                            'application/dash+xml': 'mpd'
                        };
                        if (mimeToExt[pureMime]) {
                            fileInfo.ext = mimeToExt[pureMime];
                        }
                    }
                    
                    // 过滤常见的广告碎切片或极小图标
                    if (['png', 'jpg', 'jpeg', 'gif', 'svg', 'ico', 'js', 'css', 'woff2'].includes(fileInfo.ext)) return;

                    console.log(`[DL-Omni] 捕获媒体流 (${source}):`, absUrl);

                    const headers = {
                        "Referer": window.location.href,
                        "User-Agent": navigator.userAgent,
                        "Cookie": document.cookie || ""
                    };

                    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                        window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                            event: "sniffed_resource",
                            payload: {
                                url: absUrl,
                                type: type,
                                filename: `嗅探资源 [${source}]`,
                                page_title: getPageMetadata(),
                                original_name: fileInfo.original_name,
                                ext: fileInfo.ext,
                                headers: headers
                            }
                        }).catch(err => console.error("[DL-Omni] IPC 失败:", err));
                    }
                } catch(e) {}
            }

            // ==========================================
            // 阶段一：DOM 原型链劫持
            // ==========================================
            const originalVideoSrc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
            if (originalVideoSrc) {
                Object.defineProperty(HTMLMediaElement.prototype, 'src', {
                    set: function(value) {
                        tryEmit(value, 'video', 'DOM Hook (src)');
                        return originalVideoSrc.set.call(this, value);
                    },
                    get: function() {
                        return originalVideoSrc.get.call(this);
                    }
                });
            }

            const originalSetAttribute = Element.prototype.setAttribute;
            Element.prototype.setAttribute = function(name, value) {
                if (this instanceof HTMLMediaElement && name === 'src') {
                    tryEmit(value, 'video', 'DOM Hook (setAttribute)');
                }
                return originalSetAttribute.apply(this, arguments);
            };

            // ==========================================
            // 阶段二：健壮的启发式 URL 匹配
            // ==========================================
            function heuristicMatch(url) {
                const u = url.toLowerCase();
                if ((u.includes('mypikpak.com') || u.includes('pikpak.io') || u.includes('pikpak.net')) && u.includes('/download/') && (u.includes('sign=') || u.includes('signature='))) return 'PikPak';
                if ((u.includes('aliyundrive.net') || u.includes('alipan.com')) && (u.includes('signature=') || u.includes('auth_key='))) return 'AliYunPan';
                return null;
            }

            // ==========================================
            // 阶段三：网络请求响应体解析
            // ==========================================
            async function inspectResponse(url, cloneRes, source) {
                try {
                    const hMatch = heuristicMatch(url);
                    if (hMatch) {
                        tryEmit(url, 'video', `${source} - 启发式 (${hMatch})`);
                        return;
                    }

                    const contentType = cloneRes.headers.get('content-type') || '';

                    // [修改点] 将拦截到的 contentType 传递给 tryEmit 辅助推断后缀
                    if (contentType.includes('video/') || contentType.includes('audio/') ||
                        contentType.includes('mpegurl') || contentType.includes('dash+xml') ||
                        contentType.includes('application/octet-stream')) {

                        if (contentType.includes('application/octet-stream')) {
                            if (url.includes('fid=') || url.includes('sign=') || url.includes('token=')) {
                                tryEmit(url, 'media (octet-stream)', `${source} MIME`, contentType);
                            }
                        } else {
                            tryEmit(url, contentType.split('/')[0] || 'media', `${source} MIME`, contentType);
                        }
                        return;
                    }

                    if (contentType.includes('application/json')) {
                        const text = await cloneRes.text();
                        if (!text) return;

                        try {
                            const data = JSON.parse(text);
                            const findUrl = (obj) => {
                                if (!obj || typeof obj !== 'object') return;
                                for (const key in obj) {
                                    if (typeof obj[key] === 'string' && (obj[key].startsWith('http://') || obj[key].startsWith('https://'))) {
                                        const k = key.toLowerCase();
                                        const v = obj[key].toLowerCase();

                                        const nestedMatch = heuristicMatch(obj[key]);
                                        if (nestedMatch) {
                                            tryEmit(obj[key], 'video', `${source} - API 安全解析 (${nestedMatch})`);
                                            continue;
                                        }

                                        const isGarbage = v.includes('.health') || v.includes('config') || v.includes('/log') || v.includes('/report');
                                        if (isGarbage) continue;

                                        const hasMediaExt = v.includes('.mp4') || v.includes('.m3u8') || v.includes('.flv') || v.includes('.mkv');
                                        const isPlayKey = k.includes('play') || k.includes('video') || k.includes('m3u8');

                                        if (hasMediaExt || isPlayKey) {
                                            tryEmit(obj[key], 'video', `${source} - API 安全解析`);
                                        }
                                    } else if (typeof obj[key] === 'object') {
                                        findUrl(obj[key]);
                                    }
                                }
                            };
                            findUrl(data);
                        } catch(jsonErr) {
                            const dyMatch = text.match(/"url_list"\s*:\s*\["([^"]+)"\]/);
                            if (dyMatch && dyMatch[1]) tryEmit(dyMatch[1].replace(/\\u0026/g, '&'), 'video', `${source} - 正则兜底`);
                        }
                    }
                } catch(e) {}
            }

            // ==========================================
            // 阶段四：底层 Fetch & XHR 劫持
            // ==========================================
            const originalFetch = window.fetch;
            window.fetch = async function(...args) {
                const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
                const hMatch = heuristicMatch(reqUrl);
                if (hMatch) {
                    tryEmit(reqUrl, 'video', `Fetch - 启发式 (${hMatch})`);
                }
                const response = await originalFetch.apply(this, args);
                inspectResponse(reqUrl, response.clone(), 'Fetch');
                return response;
            };

            const originalXhrOpen = XMLHttpRequest.prototype.open;
            const originalXhrSend = XMLHttpRequest.prototype.send;

            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this._reqUrl = url;
                const hMatch = heuristicMatch(url);
                if (hMatch) {
                    tryEmit(url, 'video', `XHR - 启发式 (${hMatch})`);
                }
                return originalXhrOpen.call(this, method, url, ...rest);
            };

            XMLHttpRequest.prototype.send = function(...args) {
                this.addEventListener('load', function() {
                    try {
                        const contentType = this.getResponseHeader('content-type') || '';
                        const fakeRes = {
                            headers: new Headers({ 'content-type': contentType }),
                            text: async () => {
                                if (this.responseType === '' || this.responseType === 'text') {
                                    return this.responseText;
                                } else if (this.responseType === 'json') {
                                    return typeof this.response === 'object' ? JSON.stringify(this.response) : this.response;
                                }
                                return "";
                            }
                        };
                        inspectResponse(this._reqUrl, fakeRes, 'XHR');
                    } catch(e) {}
                });
                return originalXhrSend.apply(this, args);
            };

            // ==========================================
            // 阶段五：防止链接在新窗口打开
            // ==========================================
            document.addEventListener('click', function(e) {
                let target = e.target;
                while (target && target.tagName !== 'A') {
                    target = target.parentNode;
                }
                if (target && target.tagName === 'A' && target.getAttribute('target') === '_blank') {
                    target.removeAttribute('target');
                }
            }, true);

            const originalOpen = window.open;
            window.open = function(url, target, features) {
                if (target === '_blank') target = '_self';
                return originalOpen.call(window, url, target, features);
            };

        })();
    "#;

    let app_handle_clone = app.clone();
    app.listen("sniffed_cookie", move |event| {
        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            if let (Some(domain), Some(cookie)) = (payload.get("domain").and_then(|d| d.as_str()), payload.get("cookie").and_then(|c| c.as_str())) {
                let cookie_path = get_internal_cookie_path(&app_handle_clone);
                
                let mut content = String::from("# Netscape HTTP Cookie File\n");
                content.push_str("# This file was generated by DL-Omni Internal Sniffer\n\n");
                content.push_str(&format_cookie_to_netscape(domain, cookie));

                let _ = fs::write(&cookie_path, content);
            }
        }
    });

    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url.parse().unwrap()))
        .title("DL-Omni - 资源嗅探器 (猫抓级多路引擎)")
        .inner_size(1100.0, 800.0)
        .initialization_script(init_script)
        .build()
        .map_err(|e| format!("无法创建嗅探窗口: {}", e))?;

    Ok(())
}

pub async fn stop_sniffer(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("sniffer_window") {
        win.close().map_err(|e| format!("关闭嗅探窗口失败: {}", e))?;
    }
    Ok(())
}