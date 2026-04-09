/**
 * PikPak 专属嗅探适配器
 */
class PikPakAdapter extends SnifferAdapter {
    constructor() {
        super('PikPak');
    }

    match(currentUrl) {
        return currentUrl.includes('mypikpak.com') || currentUrl.includes('pikpak.io') || currentUrl.includes('pikpak.net');
    }

    heuristicMatch(reqUrl) {
        const u = reqUrl.toLowerCase();
        // PikPak 的直链通常包含 /download/ 并带有签名参数
        if (u.includes('/download/') && (u.includes('sign=') || u.includes('signature='))) {
            return true;
        }
        return false;
    }

    interceptResponse(reqUrl, contentType, responseData) {
        // PikPak 主要依赖 heuristicMatch 拦截，也可以在此补充 JSON API 解析逻辑
        return null;
    }
}

// 注册 PikPak 适配器
window.__DL_OMNI_ADAPTERS__.push(new PikPakAdapter());