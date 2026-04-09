// 初始化全局适配器注册表
window.__DL_OMNI_ADAPTERS__ = window.__DL_OMNI_ADAPTERS__ || [];

/**
 * 嗅探器适配器基类
 * 所有特定平台的解析脚本都必须继承此基类
 */
class SnifferAdapter {
    constructor(name) {
        this.name = name;
    }

    /**
     * 检查当前网址是否适用此适配器
     * @param {string} currentUrl 当前网页的 URL
     * @returns {boolean}
     */
    match(currentUrl) {
        return false;
    }

    /**
     * 启发式 URL 检查（例如拦截带有特定参数的直接请求）
     * 用于在发起 Fetch/XHR 请求前直接判定是否为目标流媒体
     * @param {string} reqUrl 发起的网络请求 URL
     * @returns {boolean} 是否为目标媒体流
     */
    heuristicMatch(reqUrl) {
        return false;
    }

    /**
     * 处理网络请求拦截 (Fetch/XHR 的 Response)
     * 主要用于拦截 API 返回的 JSON，并从中提取真实的视频/音频直链
     * @param {string} reqUrl 请求的 API URL
     * @param {string} contentType 响应类型 (如 application/json)
     * @param {string|object} responseData 响应数据 (文本或对象)
     * @returns {string|null} 返回提取到的真实媒体流 URL，若无则返回 null
     */
    interceptResponse(reqUrl, contentType, responseData) {
        return null;
    }
}