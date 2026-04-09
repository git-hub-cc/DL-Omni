/**
 * 阿里云盘 (AliYunPan) 专属嗅探适配器
 */
class AliYunAdapter extends SnifferAdapter {
    constructor() {
        super('AliYunPan');
    }

    match(currentUrl) {
        return currentUrl.includes('aliyundrive.net') || currentUrl.includes('alipan.com');
    }

    heuristicMatch(reqUrl) {
        const u = reqUrl.toLowerCase();
        // 阿里云盘视频直链特征
        if (u.includes('signature=') || u.includes('auth_key=')) {
            // 简单的防误判：通常直链不会带有 api 的路径
            if (!u.includes('/api/')) {
                return true;
            }
        }
        return false;
    }

    interceptResponse(reqUrl, contentType, responseData) {
        if (!contentType.includes('application/json')) return null;

        try {
            const data = typeof responseData === 'string' ? JSON.parse(responseData) : responseData;
            
            // 阿里云盘获取下载链接接口的解析
            if (data && data.url && typeof data.url === 'string' && data.url.includes('auth_key=')) {
                return data.url;
            }
        } catch (e) {
            // 忽略错误
        }
        return null;
    }
}

// 注册阿里云盘适配器
window.__DL_OMNI_ADAPTERS__.push(new AliYunAdapter());