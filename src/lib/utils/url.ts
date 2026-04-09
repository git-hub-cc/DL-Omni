/**
 * 智能格式化用户输入的 URL
 * 自动去除首尾空格，并在缺少 http/https 协议头时默认补全 https://
 */
export function formatUrl(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return trimmed;
  
  // 如果不以 http:// 或 https:// 开头，则默认补充 https://
  if (!/^https?:\/\//i.test(trimmed)) {
    return `https://${trimmed}`;
  }
  
  return trimmed;
}