/**
 * 智能格式化用户输入的 URL
 * 自动去除首尾空格，并在缺少 http/https 协议头时默认补全 https://
 */
export function formatUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return trimmed;
  
  // 如果是磁力链接，保持原样返回
  if (/^magnet:/i.test(trimmed)) {
    return trimmed;
  }
  
  // 如果不以 http:// 或 https:// 开头，则默认补充 https://
  if (!/^https?:\/\//i.test(trimmed)) {
    return `https://${trimmed}`;
  }
  
  return trimmed;
}

/**
 * 从任意文本中批量提取有效的 HTTP/HTTPS 链接及磁力链接
 * @param text 包含链接的混合文本
 * @returns 提取到的有效 URL 数组
 */
export function extractUrls(text: string): string[] {
  if (!text) return [];
  // 匹配标准 http/https 链接及 magnet 链接，遇到空格、引号、括号等截断
  const urlRegex = /(https?:\/\/[^\s<]+[^<.,:;"')\]\s]|magnet:\?[^\s<"']+)/gi;
  const matches = text.match(urlRegex);
  
  // 去重并返回
  return matches ? Array.from(new Set(matches)) : [];
}

/**
 * 专门用于解析磁力链接中的 dn (display name) 参数
 * @param url 磁力链接
 * @returns 提取出的名称，若无则返回 null
 */
export function extractMagnetDn(url: string): string | null {
  const match = url.match(/[?&]dn=([^&]+)/i);
  if (match && match[1]) {
    try {
      return decodeURIComponent(match[1]);
    } catch (e) {
      return match[1];
    }
  }
  return null;
}