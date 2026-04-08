use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 初始化高级嗅探器逻辑 (基于 Tauri 2 Webview 与多层级脚本注入)
pub async fn init_sniffer(url: String, app: AppHandle) -> Result<(), String> {
    let label = "sniffer_window";

    if let Some(win) = app.get_webview_window(label) {
        let _ = win.close();
    }

    // 核心拦截脚本：实现了 MIME 拦截与 JSON API 特征提取
    let init_script = r#"
        (function() {
            console.log("[DL-Omni] 高级多层级嗅探脚本已注入，开始侦听...");
            
            // 去重池，避免同一链接高频触发 IPC
            const emittedUrls = new Set();

            function getAbsoluteUrl(url) {
                try { return new URL(url, window.location.href).href; }
                catch(e) { return url; }
            }

            // 统一的消息发送函数 (自动附带网页的 Referer 和 User-Agent 以突破防盗链)
            function tryEmit(url, type, source) {
                if (!url || typeof url !== 'string' || !url.startsWith('http')) return;
                
                const absUrl = getAbsoluteUrl(url);
                if (emittedUrls.has(absUrl)) return;
                emittedUrls.add(absUrl);

                console.log(`[DL-Omni] 捕获媒体流 (${source}):`, absUrl);

                // 组装动态 Headers
                const headers = {
                    "Referer": window.location.href,
                    "User-Agent": navigator.userAgent
                };

                if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                    window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
                        event: "sniffed_resource",
                        payload: {
                            url: absUrl,
                            type: type,
                            filename: `媒体资源 [${source}]`,
                            headers: headers // 发送给后端持久化
                        }
                    }).catch(err => console.error("[DL-Omni] IPC 失败:", err));
                }
            }

            // 检查响应类型或内容，判定是否为视频/音频
            async function inspectResponse(url, cloneRes, source) {
                try {
                    const contentType = cloneRes.headers.get('content-type') || '';
                    
                    // 【第一层】基于 MIME 类型精准拦截 (无视 URL 后缀)
                    if (contentType.includes('video/') || contentType.includes('audio/') || 
                        contentType.includes('mpegurl') || contentType.includes('dash+xml')) {
                        tryEmit(url, contentType.split('/')[0] || 'media', `${source} MIME`);
                        return;
                    }

                    // 【第二层】JSON API 特征提取 (针对抖音/B站等)
                    if (contentType.includes('application/json')) {
                        const text = await cloneRes.text();
                        
                        // 抖音特征: "url_list":["https://..."]
                        const dyMatch = text.match(/"url_list"\s*:\s*\["([^"]+)"\]/);
                        if (dyMatch && dyMatch[1]) {
                            // 修复 JSON 序列化中的 Unicode 转义字符
                            const cleanUrl = dyMatch[1].replace(/\\u0026/g, '&');
                            tryEmit(cleanUrl, 'video', `${source} - API解析 (Douyin)`);
                        }
                        
                        // B站特征: "baseUrl":"https://..." 或 "url":"https://..." (需组合判断)
                        // ... 此处可根据实际需求无限扩展特征正则表达式库
                    }
                } catch(e) {
                    // 忽略跨域或读取流报错
                }
            }

            // --- 拦截 Fetch ---
            const originalFetch = window.fetch;
            window.fetch = async function(...args) {
                const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
                const response = await originalFetch.apply(this, args);
                
                // 必须 clone response 否则会破坏原网页的读取
                inspectResponse(reqUrl, response.clone(), 'Fetch');
                return response;
            };

            // --- 拦截 XMLHttpRequest ---
            const originalXhrOpen = XMLHttpRequest.prototype.open;
            const originalXhrSend = XMLHttpRequest.prototype.send;
            
            XMLHttpRequest.prototype.open = function(method, url, ...rest) {
                this._reqUrl = url;
                return originalXhrOpen.call(this, method, url, ...rest);
            };
            
            XMLHttpRequest.prototype.send = function(...args) {
                this.addEventListener('load', function() {
                    try {
                        const contentType = this.getResponseHeader('content-type') || '';
                        
                        // 构造一个伪 Response 对象复用逻辑
                        const fakeRes = {
                            headers: new Headers({ 'content-type': contentType }),
                            text: async () => this.responseText
                        };
                        
                        inspectResponse(this._reqUrl, fakeRes, 'XHR');
                    } catch(e) {}
                });
                return originalXhrSend.apply(this, args);
            };

        })();
    "#;

    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url.parse().unwrap()))
        .title("DL-Omni - 资源嗅探器 (多层级引擎)")
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