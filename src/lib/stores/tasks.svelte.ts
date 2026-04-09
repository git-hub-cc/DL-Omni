import type { Task, MediaInfo, SniffedResource } from '$lib/types';
import { IPC } from '$lib/api/ipc';
import { configStore } from '$lib/stores/config.svelte';

class TaskStore {
  tasks = $state<Record<string, Task>>({});

  taskList = $derived(
    Object.values(this.tasks).sort((a, b) => b.created_at - a.created_at)
  );
  
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

  /**
   * 仿猫抓模板解析引擎
   * 支持占位符: [title] 网页标题, [name] 原始名, [ext] 扩展名, [time] 时间戳
   */
  parseTemplate(resource: SniffedResource): string {
    const template = configStore.settings.naming_template || '[title] - [name].[ext]';
    const now = new Date();
    const timestamp = `${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}`;
    
    let result = template
      .replace('[title]', resource.page_title || '未知网页')
      .replace('[name]', (resource.original_name || 'file').split('.')[0])
      .replace('[ext]', resource.ext || 'mp4')
      .replace('[time]', timestamp);

    // 基础清洗：防止模板配置错误导致文件名包含非法字符
    return result.replace(/[\\/:*?"<>|]/g, '_').trim();
  }

  createTempTask(url: string, title: string, httpHeaders?: string): string {
    const tempId = `temp-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
    this.add({
      id: tempId,
      url: url,
      title: title,
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
        const videoFilter = video_quality === 'best' ? 'bv*' : `bv[height<=${video_quality.replace('p', '')}]`;
        const audioFilter = audio_quality === 'best' ? 'ba' : `ba[abr<=${audio_quality.replace('k', '')}]`;
        formatId = split_audio_video ? `${videoFilter}/${audioFilter}` : `${videoFilter}+${audioFilter}/b`;
      }
      
      const title = this.tasks[tempId].title !== "解析/处理中..." ? this.tasks[tempId].title : (info.title || "未知标题");
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

  async submitNewTask(url: string, httpHeaders?: string) {
    const tempId = this.createTempTask(url, "解析/处理中...", httpHeaders);
    try {
      const info = await IPC.parseUrl(url);
      await this.commitTask(tempId, url, info, undefined, httpHeaders);
    } catch (e: any) {
      this.update(tempId, { status: 'error', title: '解析失败', error_msg: e?.toString() });
    }
  }

  async submitSniffedTask(resource: SniffedResource) {
    const headersStr = resource.headers ? JSON.stringify(resource.headers) : undefined;
    
    // 使用模板引擎生成良好的标题名
    const finalTitle = this.parseTemplate(resource);
    const tempId = this.createTempTask(resource.url, finalTitle, headersStr);
    
    try {
      const isM3u8 = resource.url.toLowerCase().includes('.m3u8');
      const fakeInfo: MediaInfo = {
        id: isM3u8 ? 'hls_stream' : 'direct_link', 
        title: finalTitle,
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