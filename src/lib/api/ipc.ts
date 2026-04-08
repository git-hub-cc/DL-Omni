import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { MediaInfo, Task, SniffedResource } from '$lib/types';
import { taskStore } from '$lib/stores/tasks.svelte';

export const IPC = {
  async parseUrl(url: string): Promise<MediaInfo> {
    return await invoke<MediaInfo>('parse_url', { url });
  },

  async createTask(
    url: string, 
    title: string, 
    thumbnail: string | undefined, 
    formatId: string,
    playlistItems?: string,
    httpHeaders?: string // 【新增】透传请求头到 Rust
  ): Promise<string> {
    return await invoke<string>('create_task', { 
      url, 
      title, 
      thumbnail, 
      formatId, 
      playlistItems,
      httpHeaders // 后端已在 payload 中接收此字段
    });
  },

  async pauseTask(taskId: string): Promise<void> {
    await invoke('pause_task', { taskId });
  },

  async resumeTask(taskId: string): Promise<void> {
    await invoke('resume_task', { taskId });
  },

  async getAllTasks(): Promise<Task[]> {
    return await invoke<Task[]>('get_all_tasks');
  },

  async cancelTask(taskId: string): Promise<void> {
    await invoke('cancel_task', { taskId });
  },

  async clearHistory(): Promise<void> {
    await invoke('clear_history');
  },

  async openFolder(): Promise<void> {
    await invoke('open_folder');
  },

  async startSniffing(url: string): Promise<void> {
    await invoke('start_sniffing', { url });
  },

  async stopSniffing(): Promise<void> {
    await invoke('stop_sniffing');
  },

  async listenProgressUpdates(): Promise<UnlistenFn> {
    return await listen<Partial<Task>[]>('batch_progress_update', (event) => {
      taskStore.batchUpdateProgress(event.payload);
    });
  },

  async listenTaskError(): Promise<UnlistenFn> {
    return await listen<{ id: string, error: string }>('task_error', (event) => {
      const { id, error } = event.payload;
      taskStore.update(id, { status: 'error', error_msg: error });
    });
  },

  // 【修改】回调参数使用规范的 SniffedResource 类型
  async listenSniffedResources(callback: (resource: SniffedResource) => void): Promise<UnlistenFn> {
    return await listen<SniffedResource>('sniffed_resource', (event) => {
      callback(event.payload);
    });
  }
};