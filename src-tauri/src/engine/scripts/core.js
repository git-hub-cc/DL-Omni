/**
 * DL-Omni 嗅探核心引擎
 * 负责底层 Hook (DOM, XHR, Fetch) 并将数据分发给匹配的适配器进行解析
 */
(function() {
    console.log("[DL-Omni] 核心引擎加载，初始化适配器...");
    
    const emittedUrls = new Set();

    // ==========================================
    // 核心功能：Cookie 同步与提取网页元数据
    // ==========================================
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

    window.addEventListener('load', syncCookieToBackend);
    setInterval(syncCookieToBackend, 5000);

    function getPageMetadata() {
        try {
            const ogTitle = document.querySelector('meta[property="og:title"]');
            let title = ogTitle ? ogTitle.getAttribute('content') : document.title;
            
            if (title) {
                // 清理各大网盘常见的分享后缀废话，防止污染文件名和搜索干扰
                title = title.replace(/\s*is shared on PikPak.*$/i, '');
                title = title.replace(/\s*-\s*阿里云盘分享.*$/i, '');
                title = title.replace(/\s*-\s*夸克网盘分享.*$/i, '');
            }
            
            return title ? title.trim() : '未知网页';
        } catch (e) {
            return '未知网页';
        }
    }

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

    // ==========================================
    // 暴露给适配器的核心 API
    // ==========================================
    window.__DL_OMNI_CORE__ = {
        tryEmit: function(url, type, source, mimeType = null) {
            if (!url || typeof url !== 'string' || !url.startsWith('http')) return;

            try {
                const absUrl = getAbsoluteUrl(url);

                if (emittedUrls.has(absUrl)) return;
                emittedUrls.add(absUrl);

                let fileInfo = extractFileInfo(absUrl);
                
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
    };

    // ==========================================
    // 适配器调度系统
    // ==========================================
    function getActiveAdapters(url) {
        return (window.__DL_OMNI_ADAPTERS__ || []).filter(adapter => adapter.match(url));
    }

    async function inspectResponse(url, cloneRes, source) {
        try {
            const currentUrl = window.location.href;
            const adapters = getActiveAdapters(currentUrl);
            
            // 启发式匹配检查
            for (const adapter of adapters) {
                if (adapter.heuristicMatch(url)) {
                    window.__DL_OMNI_CORE__.tryEmit(url, 'video', `${source} - ${adapter.name} 启发式`);
                    return;
                }
            }

            const contentType = cloneRes.headers.get('content-type') || '';

            // MIME 直接拦截
            if (contentType.includes('video/') || contentType.includes('audio/') ||
                contentType.includes('mpegurl') || contentType.includes('dash+xml') ||
                contentType.includes('application/octet-stream')) {

                if (contentType.includes('application/octet-stream')) {
                    if (url.includes('fid=') || url.includes('sign=') || url.includes('token=')) {
                        window.__DL_OMNI_CORE__.tryEmit(url, 'media (octet-stream)', `${source} MIME`, contentType);
                    }
                } else {
                    window.__DL_OMNI_CORE__.tryEmit(url, contentType.split('/')[0] || 'media', `${source} MIME`, contentType);
                }
                return;
            }

            // API 响应体验证 (转交适配器处理)
            if (contentType.includes('application/json') || contentType.includes('text/')) {
                const text = await cloneRes.text();
                if (!text) return;

                for (const adapter of adapters) {
                    const resultUrl = adapter.interceptResponse(url, contentType, text);
                    if (resultUrl) {
                        window.__DL_OMNI_CORE__.tryEmit(resultUrl, 'video', `${source} - ${adapter.name} 接口解析`);
                        return; // 一旦有适配器处理成功，则阻断后续执行
                    }
                }
            }
        } catch(e) {}
    }

    // ==========================================
    // 底层 Hook (DOM / Fetch / XHR)
    // ==========================================
    const originalVideoSrc = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, 'src');
    if (originalVideoSrc) {
        Object.defineProperty(HTMLMediaElement.prototype, 'src', {
            set: function(value) {
                window.__DL_OMNI_CORE__.tryEmit(value, 'video', 'DOM Hook (src)');
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
            window.__DL_OMNI_CORE__.tryEmit(value, 'video', 'DOM Hook (setAttribute)');
        }
        return originalSetAttribute.apply(this, arguments);
    };

    const originalFetch = window.fetch;
    window.fetch = async function(...args) {
        const reqUrl = typeof args[0] === 'string' ? args[0] : (args[0] && args[0].url ? args[0].url : '');
        
        // 启发式预检
        const currentUrl = window.location.href;
        const adapters = getActiveAdapters(currentUrl);
        for (const adapter of adapters) {
            if (adapter.heuristicMatch(reqUrl)) {
                window.__DL_OMNI_CORE__.tryEmit(reqUrl, 'video', `Fetch - ${adapter.name} 启发式`);
            }
        }

        const response = await originalFetch.apply(this, args);
        inspectResponse(reqUrl, response.clone(), 'Fetch');
        return response;
    };

    const originalXhrOpen = XMLHttpRequest.prototype.open;
    const originalXhrSend = XMLHttpRequest.prototype.send;

    XMLHttpRequest.prototype.open = function(method, url, ...rest) {
        this._reqUrl = url;
        const currentUrl = window.location.href;
        const adapters = getActiveAdapters(currentUrl);
        for (const adapter of adapters) {
            if (adapter.heuristicMatch(url)) {
                window.__DL_OMNI_CORE__.tryEmit(url, 'video', `XHR - ${adapter.name} 启发式`);
            }
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
    // 防止链接在新窗口打开 (保持在嗅探器沙盒内)
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