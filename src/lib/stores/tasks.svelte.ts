import type { Task, TaskStatus } from '$lib/types';
import { IPC } from '$lib/api/ipc';
import { configStore } from '$lib/stores/config.svelte';

/**
 * 全局任务池状态管理 (Svelte 5 Runes)
 * 使用类封装以保证单一实例和严密的业务逻辑闭环
 */
class TaskStore {
  // 核心状态：使用原生对象 Record 以保证 Svelte 5 更好的响应式支持 (O(1) 查找)
  tasks = $state<Record<string, Task>>({});

  // --- 派生视图 (Derived Views) ---
  
  // 转换为数组以便于视图渲染
  taskList = $derived(Object.values(this.tasks));
  
  // 过滤视图：下载中/排队中/合并中
  activeTasks = $derived(
    this.taskList.filter(t => 
      t.status === 'downloading' || t.status === 'pending' || t.status === 'merging'
    )
  );

  // 过滤视图：已完成
  completedTasks = $derived(
    this.taskList.filter(t => t.status === 'completed')
  );

  // 过滤视图：错误或暂停
  pausedOrErrorTasks = $derived(
    this.taskList.filter(t => t.status === 'paused' || t.status === 'error')
  );

  // --- 核心操作方法 ---

  /**
   * 初始化/全量替换任务池 (用于应用启动时从本地 DB 恢复数据)
   */
  init(initialTasks: Task[]) {
    // 重新创建一个新对象以触发彻底引用更新，避免潜在的遗留状态
    const newTasks: Record<string, Task> = {};
    for (const task of initialTasks) {
      newTasks[task.id] = task;
    }
    this.tasks = newTasks;
  }

  /**
   * 新增任务
   */
  add(task: Task) {
    this.tasks[task.id] = task;
  }

  /**
   * 细粒度更新单一任务属性 (不替换整个对象，极少触发额外 Reflow)
   */
  update(id: string, partial: Partial<Task>) {
    if (this.tasks[id]) {
      // 细粒度修改即可触发对象属性代理更新
      this.tasks[id] = { ...this.tasks[id], ...partial };
    }
  }

  /**
   * 批量更新进度 (应对后端节流后的批量事件推送)
   * 负载格式: [{ id, downloaded_bytes, speed, eta, status }]
   */
  batchUpdateProgress(updates: Partial<Task>[]) {
    for (const update of updates) {
      if (update.id) {
        this.update(update.id, update);
      }
    }
  }

  /**
   * 移除任务
   */
  remove(id: string) {
    delete this.tasks[id];
  }

  /**
   * 全局提交新任务 (从任意页面发起解析并下载的完整闭环)
   */
  async submitNewTask(url: string) {
    const tempId = `temp-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;

    this.add({
      id: tempId,
      url: url,
      title: "解析中...",
      thumbnail: undefined,
      status: 'pending',
      format_id: '',
      total_bytes: 0,
      downloaded_bytes: 0,
      speed: 0,
      eta: 0,
      created_at: Date.now(),
      error_msg: undefined
    });

    try {
      console.log('解析中...', url);
      const info = await IPC.parseUrl(url);
      console.log('解析完成', info);
      
      if (!this.tasks[tempId]) {
        console.log('临时任务已被删除，取消创建任务');
        return; 
      }
      
      const { split_audio_video, video_quality, audio_quality } = configStore.settings;

      // 构建视频画质过滤条件
      const videoFilter = video_quality === 'best'
        ? 'bv*'
        : `bv[height<=${video_quality.replace('p', '')}]`;

      // 构建音频画质过滤条件
      const audioFilter = audio_quality === 'best'
        ? 'ba'
        : `ba[abr<=${audio_quality.replace('k', '')}]`;

      // split_audio_video=true → 分开下载，保留独立音视频文件
      // split_audio_video=false → 自动合并为单一文件（最佳格式优先）
      const formatId = split_audio_video
        ? `${videoFilter}/${audioFilter}`           // 分开：只取视频流或音频流（不合并）
        : `${videoFilter}+${audioFilter}/b`;        // 合并：视频+音频，回退到最佳单一格式
      
      const title = info.title || "未知标题";
      const thumbnail: string | undefined = info.thumbnail || undefined;
      const taskId = await IPC.createTask(url, title, thumbnail, formatId);
      
      if (this.tasks[tempId]) {
        this.remove(tempId);
        this.add({
          id: taskId,
          url: url,
          title: title,
          thumbnail: thumbnail,
          status: 'pending',
          format_id: formatId,
          total_bytes: 0,
          downloaded_bytes: 0,
          speed: 0,
          eta: 0,
          created_at: Date.now(),
          error_msg: undefined
        });
      }
    } catch (e: any) {
      console.error('解析/生成任务失败:', e);
      if (this.tasks[tempId]) {
        this.update(tempId, { 
          status: 'error', 
          title: '解析失败',
          error_msg: e?.toString() || '未知错误'
        });
      }
    }
  }
}

// 导出全局单例
export const taskStore = new TaskStore();