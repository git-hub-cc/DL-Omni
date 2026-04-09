/**
 * 抖音 (Douyin) 专属嗅探适配器
 */
class DouyinAdapter extends SnifferAdapter {
    constructor() {
        super('Douyin');
    }

    match(currentUrl) {
        return currentUrl.includes('douyin.com') || currentUrl.includes('iesdouyin.com');
    }

    heuristicMatch(reqUrl) {
        // 抖音的视频流通常不在请求 URL 中直接暴露清晰的特征，主要依赖 API 解析
        return false;
    }

    interceptResponse(reqUrl, contentType, responseData) {
        if (!contentType.includes('application/json')) return null;

        try {
            const data = typeof responseData === 'string' ? JSON.parse(responseData) : responseData;
            
            // 针对抖音 API 的专属解析逻辑：提取 aweme_list 中的所有视频无水印直链
            if (data && data.aweme_list && Array.isArray(data.aweme_list)) {
                for (const aweme of data.aweme_list) {
                    if (aweme.video && aweme.video.play_addr && aweme.video.play_addr.url_list && aweme.video.play_addr.url_list.length > 0) {
                        const url = aweme.video.play_addr.url_list[0];
                        // 由于抖音 API 可能一次返回多个视频，直接在此处调用核心引擎发射事件
                        if (window.__DL_OMNI_CORE__) {
                            window.__DL_OMNI_CORE__.tryEmit(url.replace(/\\u0026/g, '&'), 'video', 'Douyin API');
                        }
                    }
                }
            }
        } catch (e) {
            // 解析失败时的正则兜底：尝试直接在响应文本中匹配 url_list
            const dyMatch = String(responseData).match(/"url_list"\s*:\s*\["([^"]+)"\]/);
            if (dyMatch && dyMatch[1]) {
                return dyMatch[1].replace(/\\u0026/g, '&');
            }
        }
        
        return null;
    }
}

// 注册抖音适配器
window.__DL_OMNI_ADAPTERS__.push(new DouyinAdapter());