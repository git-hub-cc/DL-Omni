import type { Config } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';

class ConfigStore {
  // 增加模板和正则的默认极简配置
  settings = $state<Config>({
    default_download_path: '',
    max_concurrent_tasks: 3,
    max_threads_per_task: 16,
    proxy_url: '',
    theme: 'system',
    split_audio_video: false, 
    video_quality: 'best',    
    audio_quality: 'best',    
    use_cookie: false,         // 修改：是否使用内置浏览器的 Cookie
    include_metadata: false,
    // 猫抓风格的默认占位符模板，优先取网页标题，没有则取 URL 原始名字
    naming_template: '[title] - [name].[ext]',
    // 默认屏蔽常见广告或无用统计域名
    sniff_blacklist: 'google-analytics|doubleclick\\.net|\\.log$|\\.health$', 
  });

  /**
   * 初始化应用配置
   */
  async init() {
    try {
      const savedConfig = await invoke<Config>('get_config');
      Object.assign(this.settings, savedConfig);
    } catch (e) {
      console.error('Failed to fetch config from backend:', e);
    }
  }

  /**
   * 更新配置 (触发 Tauri 写入 config.json)
   */
  async update(partial: Partial<Config>) {
    Object.assign(this.settings, partial);
    try {
      await invoke('update_config', { new_config: $state.snapshot(this.settings) });
    } catch (e) {
      console.error('Failed to update config:', e);
    }
  }
}

export const configStore = new ConfigStore();