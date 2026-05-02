import { invoke } from '@tauri-apps/api/core';
import type { MediaInfo, Task, SniffedResource, TaskProgressUpdate, TaskErrorPayload } from '$lib/types';
import { taskStore } from '$lib/stores/tasks.svelte';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

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
    httpHeaders?: string 
  ): Promise<string> {
    return await invoke<string>('create_task', { 
      url, 
      title, 
      thumbnail, 
      format_id: formatId, 
      playlist_items: playlistItems,
      http_headers: httpHeaders 
    });
  },

  async pauseTask(taskId: string): Promise<void> {
    await invoke('pause_task', { task_id: taskId });
  },

  async resumeTask(taskId: string): Promise<void> {
    await invoke('resume_task', { task_id: taskId });
  },

  async getAllTasks(): Promise<Task[]> {
    return await invoke<Task[]>('get_all_tasks');
  },

  async cancelTask(taskId: string): Promise<void> {
    await invoke('cancel_task', { task_id: taskId });
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
    return await listen<TaskProgressUpdate[]>('batch_progress_update', (event) => {
      // 防御性判断，防止后端在高并发下抛出空数据导致前端渲染白屏
      if (event.payload && Array.isArray(event.payload)) {
        taskStore.batchUpdateProgress(event.payload);
      }
    });
  },

  async listenTaskError(): Promise<UnlistenFn> {
    return await listen<TaskErrorPayload>('task_error', (event) => {
      // 严格判空清洗
      if (event.payload && event.payload.id && event.payload.error) {
        const { id, error } = event.payload;
        taskStore.update(id, { status: 'error', error_msg: error });
      }
    });
  },

  async listenSniffedResources(callback: (resource: SniffedResource) => void): Promise<UnlistenFn> {
    return await listen<SniffedResource>('sniffed_resource', (event) => {
      if (event.payload && event.payload.url) {
        callback(event.payload);
      }
    });
  },

  async listenSnifferClosed(callback: () => void): Promise<UnlistenFn> {
    return await listen('sniffer_window_closed', () => {
      callback();
    });
  }
};