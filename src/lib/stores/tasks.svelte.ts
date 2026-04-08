import type { Task, MediaInfo, SniffedResource } from '$lib/types';
import { IPC } from '$lib/api/ipc';
import { configStore } from '$lib/stores/config.svelte';

class TaskStore {
  tasks = $state<Record<string, Task>>({});

  taskList = $derived(Object.values(this.tasks));
  
  activeTasks = $derived(
    this.taskList.filter(t => 
      t.status === 'downloading' || t.status === 'pending' || t.status === 'merging'
    )
  );

  completedTasks = $derived(
    this.taskList.filter(t => t.status === 'completed')
  );

  pausedOrErrorTasks = $derived(
    this.taskList.filter(t => t.status === 'paused' || t.status === 'error')
  );

  init(initialTasks: Task[]) {
    const newTasks: Record<string, Task> = {};
    for (const task of initialTasks) {
      newTasks[task.id] = task;
    }
    this.tasks = newTasks;
  }

  add(task: Task) {
    this.tasks[task.id] = task;
  }

  update(id: string, partial: Partial<Task>) {
    if (this.tasks[id]) {
      this.tasks[id] = { ...this.tasks[id], ...partial };
    }
  }

  batchUpdateProgress(updates: Partial<Task>[]) {
    for (const update of updates) {
      if (update.id) {
        this.update(update.id, update);
      }
    }
  }

  remove(id: string) {
    delete this.tasks[id];
  }

  // --- 支持直链与合集交互的新版任务流 ---

  createTempTask(url: string, httpHeaders?: string): string {
    const tempId = `temp-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
    this.add({
      id: tempId,
      url: url,
      title: "解析/处理中...",
      thumbnail: undefined,
      status: 'pending',
      format_id: '',
      http_headers: httpHeaders, 
      total_bytes: 0,
      downloaded_bytes: 0,
      speed: 0,
      eta: 0,
      created_at: Date.now(),
      error_msg: undefined
    });
    return tempId;
  }

  async commitTask(
    tempId: string, 
    url: string, 
    info: MediaInfo, 
    playlistItems?: string,
    httpHeaders?: string
  ) {
    try {
      if (!this.tasks[tempId]) return;
      
      const { split_audio_video, video_quality, audio_quality } = configStore.settings;

      let formatId = 'direct'; 

      if (info.id !== 'direct_link') {
        const videoFilter = video_quality === 'best'
          ? 'bv*'
          : `bv[height<=${video_quality.replace('p', '')}]`;

        const audioFilter = audio_quality === 'best'
          ? 'ba'
          : `ba[abr<=${audio_quality.replace('k', '')}]`;

        formatId = split_audio_video
          ? `${videoFilter}/${audioFilter}`
          : `${videoFilter}+${audioFilter}/b`;
      }
      
      const title = info.title || "未知标题";
      const thumbnail: string | undefined = info.thumbnail || undefined;
      
      const taskId = await IPC.createTask(url, title, thumbnail, formatId, playlistItems, httpHeaders);
      
      if (this.tasks[tempId]) {
        this.remove(tempId);
        this.add({
          id: taskId,
          url: url,
          title: title,
          thumbnail: thumbnail,
          status: 'pending',
          format_id: formatId,
          playlist_items: playlistItems,
          http_headers: httpHeaders, 
          total_bytes: 0,
          downloaded_bytes: 0,
          speed: 0,
          eta: 0,
          created_at: Date.now(),
          error_msg: undefined
        });
      }
    } catch (e: any) {
      console.error('生成任务失败:', e);
      if (this.tasks[tempId]) {
        this.update(tempId, { 
          status: 'error', 
          title: '创建任务失败',
          error_msg: e?.toString() || '未知错误'
        });
      }
    }
  }

  // 常规前端解析入口 (手动提交链接)
  async submitNewTask(url: string, httpHeaders?: string) {
    const tempId = this.createTempTask(url, httpHeaders);
    try {
      const info = await IPC.parseUrl(url);
      await this.commitTask(tempId, url, info, undefined, httpHeaders);
    } catch (e: any) {
      this.update(tempId, { status: 'error', title: '解析失败', error_msg: e?.toString() });
    }
  }

  // 【修改】嗅探直链专用入口：跳过 IPC.parseUrl，增加 m3u8 格式判定
  async submitSniffedTask(resource: SniffedResource) {
    const headersStr = resource.headers ? JSON.stringify(resource.headers) : undefined;
    const tempId = this.createTempTask(resource.url, headersStr);
    
    try {
      // 判断嗅探到的链接是否为 m3u8 (HLS 播放列表)
      const isM3u8 = resource.url.toLowerCase().includes('.m3u8');

      // 伪造 MediaInfo。
      // 如果是 m3u8，给一个非 'direct_link' 的 ID，使得 commitTask 使用普通的画质拼接格式 (走 yt-dlp)；
      // 否则赋予 'direct_link' 标识，强制走原生直链下载引擎。
      const fakeInfo: MediaInfo = {
        id: isM3u8 ? 'hls_stream' : 'direct_link', 
        title: resource.filename || '嗅探到的媒体文件',
        duration: 0,
        thumbnail: '',
        formats: []
      };

      await this.commitTask(tempId, resource.url, fakeInfo, undefined, headersStr);
    } catch (e: any) {
      console.error('提交嗅探任务失败:', e);
      if (this.tasks[tempId]) {
        this.update(tempId, { 
          status: 'error', 
          title: '创建任务失败',
          error_msg: e?.toString() || '未知错误' 
        });
      }
    }
  }
}

export const taskStore = new TaskStore();