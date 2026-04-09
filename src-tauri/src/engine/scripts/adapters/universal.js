/**
 * 通用兜底嗅探适配器 (Universal)
 * 用于处理没有编写专门适配器的所有其他网站
 */
class UniversalAdapter extends SnifferAdapter {
    constructor() {
        super('Universal');
    }

    match(currentUrl) {
        // 兜底适配器永远匹配
        return true;
    }

    heuristicMatch(reqUrl) {
        const u = reqUrl.toLowerCase();
        // 匹配常见的流媒体文件后缀
        if (u.includes('.m3u8')) return true;
        if (u.includes('.mp4') && !u.includes('blank.mp4')) return true;
        if (u.includes('.flv')) return true;
        
        return false;
    }

    interceptResponse(reqUrl, contentType, responseData) {
        if (!contentType.includes('application/json') && !contentType.includes('text/')) return null;

        try {
            const data = typeof responseData === 'string' ? JSON.parse(responseData) : responseData;
            let foundUrl = null;

            // 深度递归遍历 JSON 对象，寻找潜在的媒体链接
            const findUrl = (obj) => {
                if (!obj || typeof obj !== 'object') return;
                
                for (const key in obj) {
                    if (typeof obj[key] === 'string' && (obj[key].startsWith('http://') || obj[key].startsWith('https://'))) {
                        const k = key.toLowerCase();
                        const v = obj[key].toLowerCase();

                        // 过滤常见的垃圾上报/配置链接
                        const isGarbage = v.includes('.health') || v.includes('config') || v.includes('/log') || v.includes('/report');
                        if (isGarbage) continue;

                        // 根据链接后缀或键名推断是否为媒体链接
                        const hasMediaExt = v.includes('.mp4') || v.includes('.m3u8') || v.includes('.flv') || v.includes('.mkv');
                        const isPlayKey = k.includes('play') || k.includes('video') || k.includes('m3u8');

                        if (hasMediaExt || isPlayKey) {
                            foundUrl = obj[key];
                            // 一旦找到，直接发射，不中断循环以捕获可能存在的多个链接
                            if (window.__DL_OMNI_CORE__) {
                                window.__DL_OMNI_CORE__.tryEmit(foundUrl, 'video', 'Universal API 解析');
                            }
                        }
                    } else if (typeof obj[key] === 'object') {
                        findUrl(obj[key]); // 递归搜索
                    }
                }
            };

            findUrl(data);
            
        } catch (e) {
            // 如果 JSON 解析失败，则放弃
        }
        
        return null;
    }
}

// 确保通用适配器最后注册，作为所有拦截的最后一道防线
window.__DL_OMNI_ADAPTERS__.push(new UniversalAdapter());