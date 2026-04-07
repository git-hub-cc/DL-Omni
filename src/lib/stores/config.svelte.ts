import type { Config } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';

class ConfigStore {
  // 默认极简配置
  settings = $state<Config>({
    default_download_path: '', // 需在应用初始化时调用 Rust 获取系统默认 Downloads 目录
    max_concurrent_tasks: 3,
    max_threads_per_task: 16,
    proxy_url: '',
    theme: 'system',
    split_audio_video: false, // 默认合并下载（音视频合为一个文件）
    video_quality: 'best',    // 默认最高画质
    audio_quality: 'best',    // 默认最高音质
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
      await invoke('update_config', { newConfig: $state.snapshot(this.settings) });
    } catch (e) {
      console.error('Failed to update config:', e);
    }
  }
}

export const configStore = new ConfigStore();