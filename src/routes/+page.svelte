<script lang="ts">
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import { IPC } from '$lib/api/ipc';
  import { formatUrl, extractUrls } from '$lib/utils/url';
  import ProgressBar from '$lib/components/ui/ProgressBar.svelte';
  import Modal from '$lib/components/ui/Modal.svelte';
  import type { MediaInfo } from '$lib/types';
  import { goto } from '$app/navigation';

  let activeTab = $state<'all' | 'active' | 'pausedOrError'>('all');
  let showNewTaskModal = $state(false);
  
  // URL 输入流状态
  let inputUrl = $state('');
  let isParsing = $state(false);
  let parseError = $state('');

  let parsedInfo = $state<MediaInfo | null>(null);
  let showPlaylistModal = $state(false);
  let selectedItems = $state<Set<number>>(new Set());

  // 【新增】合集选择弹窗的分页逻辑与状态
  let currentPage = $state(1);
  const pageSize = 10;

  let totalPages = $derived(
    parsedInfo?.playlist_entries ? Math.ceil(parsedInfo.playlist_entries.length / pageSize) : 0
  );

  let paginatedEntries = $derived(
    parsedInfo?.playlist_entries?.slice((currentPage - 1) * pageSize, currentPage * pageSize) || []
  );

  let displayTasks = $derived.by(() => {
    switch (activeTab) {
      case 'active': return taskStore.activeTasks;
      case 'pausedOrError': return taskStore.pausedOrErrorTasks;
      default: return taskStore.taskList.filter(t => t.status !== 'completed');
    }
  });

  // 辅助函数：统一根据元素及索引获取合集子项的绑定 ID
  function getEntryId(entry: any, index: number): number {
    return entry.playlist_index || (index + 1);
  }

  async function handleParse() {
    if (!inputUrl) return;
    
    // 调用智能批量提取工具
    const urls = extractUrls(inputUrl);

    if (urls.length === 0) {
      // 若正则未匹配到，尝试作为单个域名兜底补全
      const singleUrl = formatUrl(inputUrl);
      if (singleUrl) {
        urls.push(singleUrl);
      } else {
        parseError = '未检测到有效的流媒体链接';
        return;
      }
    }
    
    parseError = '';
    isParsing = true;

    // 分支 1：批量解析模式
    if (urls.length > 1) {
      showNewTaskModal = false;
      taskStore.submitBatchTasks(urls, undefined);
      inputUrl = '';
      isParsing = false;
      return;
    }
    
    // 分支 2：单链接解析模式（保留原有合集弹窗逻辑）
    const finalUrl = urls[0];
    try {
      const info = await IPC.parseUrl(finalUrl);
      parsedInfo = info;
      
      if (info.playlist_entries && info.playlist_entries.length > 1) {
        showNewTaskModal = false;
        // 初始化全部选中
        selectedItems = new Set(info.playlist_entries.map((e, i) => getEntryId(e, i)));
        currentPage = 1;
        showPlaylistModal = true;
      } else {
        showNewTaskModal = false;
        const tempId = taskStore.createTempTask(finalUrl, "解析/处理中...");
        await taskStore.commitTask(tempId, finalUrl, info, undefined, undefined);
        inputUrl = '';
      }
    } catch (e: any) {
      // 常规解析失败时，取消强制入队，直接带参数跳转到嗅探页面
      console.warn('常规解析失败，引导跳转至嗅探页面:', e);
      showNewTaskModal = false;
      goto(`/sniffer?url=${encodeURIComponent(finalUrl)}`);
      inputUrl = '';
    } finally {
      isParsing = false;
    }
  }

  async function handleCommitPlaylist() {
    if (!parsedInfo || selectedItems.size === 0) return;
    
    const itemsArray = Array.from(selectedItems).sort((a, b) => a - b);
    const playlistItemsStr = itemsArray.join(',');
    
    showPlaylistModal = false;
    const tempId = taskStore.createTempTask(inputUrl, "解析/处理中...");
    await taskStore.commitTask(tempId, inputUrl, parsedInfo, playlistItemsStr, undefined);
    
    inputUrl = '';
    parsedInfo = null;
    selectedItems.clear();
    currentPage = 1;
  }

  function toggleSelectAll() {
    if (!parsedInfo?.playlist_entries) return;
    
    const allIds = parsedInfo.playlist_entries.map((e, i) => getEntryId(e, i));
    
    // 严格反选逻辑：对所有项目进行状态翻转
    allIds.forEach(id => {
      if (selectedItems.has(id)) {
        selectedItems.delete(id);
      } else {
        selectedItems.add(id);
      }
    });
    selectedItems = new Set(selectedItems); // 触发响应式更新并修复之前 clear() 不触发更新的问题
  }

  function toggleSelectCurrentPage() {
    if (!parsedInfo?.playlist_entries) return;
    
    const currentIds = paginatedEntries.map((e, i) => {
        const globalIndex = (currentPage - 1) * pageSize + i;
        return getEntryId(e, globalIndex);
    });
    
    // 严格反选逻辑：仅对当前页项目进行状态翻转
    currentIds.forEach(id => {
        if (selectedItems.has(id)) {
            selectedItems.delete(id);
        } else {
            selectedItems.add(id);
        }
    });
    selectedItems = new Set(selectedItems); // 触发响应式更新
  }

  function toggleItem(idx: number) {
    if (selectedItems.has(idx)) {
      selectedItems.delete(idx);
    } else {
      selectedItems.add(idx);
    }
    selectedItems = new Set(selectedItems);
  }

  async function handleToggleTask(taskId: string, status: string) {
    try {
      if (status === 'paused' || status === 'error') {
        taskStore.update(taskId, { status: 'pending' });
        await IPC.resumeTask(taskId);
      } else {
        taskStore.update(taskId, { status: 'paused' });
        await IPC.pauseTask(taskId);
      }
    } catch (e) { console.error('操作任务状态失败:', e); }
  }

  async function handleDeleteTask(taskId: string) {
    try {
      taskStore.remove(taskId);
      await IPC.cancelTask(taskId);
    } catch (e) { console.error('删除任务失败:', e); }
  }

  function isAuthTask(headersStr?: string): boolean {
    if (!headersStr) return false;
    try {
      const headers = JSON.parse(headersStr);
      return Object.keys(headers).some(k => k.toLowerCase() === 'cookie');
    } catch (e) {
      return false;
    }
  }
</script>

<div class="h-full flex flex-col relative">
  <header class="shrink-0 px-6 py-4 flex items-center justify-between border-b border-zinc-800/50">
    <div class="flex space-x-1 bg-zinc-800/50 p-1 rounded-lg">
      {#each [
        { id: 'all', label: '全部任务' },
        { id: 'active', label: '下载中' },
        { id: 'pausedOrError', label: '已暂停/错误' }
      ] as tab}
        <button
          class="px-4 py-1.5 text-xs font-medium rounded-md transition-colors {activeTab === tab.id ? 'bg-zinc-700 text-zinc-100 shadow-sm' : 'text-zinc-400 hover:text-zinc-200'}"
          onclick={() => activeTab = tab.id as any}
        >
          {tab.label}
        </button>
      {/each}
    </div>

    <button
      class="flex items-center space-x-1 px-3 py-1.5 bg-accent-blue text-white text-xs font-medium rounded-lg hover:bg-blue-600 transition-colors shadow-sm"
      onclick={() => { showNewTaskModal = true; parseError = ''; }}
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/></svg>
      <span>新建下载</span>
    </button>
  </header>

  <div class="flex-1 overflow-y-auto p-4 space-y-3">
    {#if displayTasks.length === 0}
      <div class="h-full flex flex-col items-center justify-center text-zinc-500 space-y-2">
        <svg class="w-12 h-12 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
        <p class="text-sm">暂无任务</p>
      </div>
    {:else}
      {#each displayTasks as task (task.id)}
        <div class="group flex items-center p-3 bg-zinc-800/20 hover:bg-zinc-800/50 border border-zinc-800 rounded-xl transition-colors">
          <div class="w-20 h-14 shrink-0 bg-zinc-800 rounded-md overflow-hidden mr-4 relative">
            {#if task.thumbnail}
              <img src={task.thumbnail.replace('http://', 'https://')} alt="cover" class="w-full h-full object-cover" />
            {:else}
              <div class="w-full h-full flex items-center justify-center text-zinc-600">
                <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/></svg>
              </div>
            {/if}
            
            {#if task.playlist_items}
              <div class="absolute bottom-1 right-1 bg-black/70 px-1 rounded text-[9px] font-mono border border-zinc-700/50">合集</div>
            {/if}
            
            {#if task.http_headers}
              {#if isAuthTask(task.http_headers)}
                <div class="absolute top-1 left-1 bg-purple-500/80 px-1 rounded text-[9px] font-medium border border-purple-400/50 text-white shadow-sm flex items-center space-x-0.5">
                  <svg class="w-2.5 h-2.5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/></svg>
                  <span>专属鉴权</span>
                </div>
              {:else}
                <div class="absolute top-1 left-1 bg-emerald-500/80 px-1 rounded text-[9px] font-mono border border-emerald-400/50 text-white shadow-sm">
                  防盗链
                </div>
              {/if}
            {/if}
          </div>

          <div class="flex-1 min-w-0 pr-4">
            <h4 class="text-sm font-medium text-zinc-200 truncate mb-2">{task.title}</h4>
            <ProgressBar
              progress={task.total_bytes > 0 ? task.downloaded_bytes / task.total_bytes : task.downloaded_bytes / 100}
              speedText={task.speed > 0 ? (task.speed / 1024 / 1024).toFixed(2) + " MB/s" : (task.status === 'downloading' ? "测速中..." : "")}
              etaText={task.eta > 0 ? task.eta + "s" : ""}
              sizeText={task.total_bytes > 0 ? (task.total_bytes / 1024 / 1024).toFixed(1) + " MB" : ""}
              status={task.status}
            />
          </div>

          <div class="shrink-0 flex items-center space-x-2 opacity-0 group-hover:opacity-100 transition-opacity">
            {#if task.status !== 'completed'}
              <button
                class="w-8 h-8 flex items-center justify-center rounded-full bg-zinc-700/50 hover:bg-zinc-600 text-zinc-300"
                aria-label="暂停或恢复任务"
                title="暂停 / 恢复"
                onclick={() => handleToggleTask(task.id, task.status)}
              >
                {#if task.status === 'paused' || task.status === 'error'}
                  <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
                {:else}
                  <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                {/if}
              </button>
            {/if}
            <button
              class="w-8 h-8 flex items-center justify-center rounded-full bg-zinc-700/50 hover:bg-red-500/80 text-zinc-300 hover:text-white"
              aria-label="删除任务"
              title="删除任务"
              onclick={() => handleDeleteTask(task.id)}
            >
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <Modal show={showNewTaskModal} title="新建下载任务" onclose={() => showNewTaskModal = false}>
    <div class="space-y-4">
      <div class="flex flex-col space-y-3">
        <textarea
          bind:value={inputUrl}
          placeholder="粘贴单个视频链接，或批量粘贴包含多个链接的文本内容..."
          class="w-full bg-zinc-950 border border-zinc-700 focus:border-accent-blue rounded-lg px-4 py-3 text-sm text-zinc-100 outline-none transition-colors resize-y min-h-[100px] max-h-48"
          onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey && !isParsing) { e.preventDefault(); handleParse(); } }}
        ></textarea>
        <div class="flex justify-end">
          <button
            class="px-6 py-2 bg-accent-blue hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors disabled:opacity-50"
            onclick={handleParse}
            disabled={!inputUrl || isParsing}
          >
            {isParsing ? '解析中...' : '解析并提取'}
          </button>
        </div>
      </div>
      
      {#if parseError}
        <div class="text-xs text-red-400 bg-red-400/10 p-2 rounded border border-red-400/20 break-words">
          {parseError}
        </div>
      {/if}
      <div class="text-[11px] text-zinc-500">
        💡 提示：支持一键混贴包含多个链接的文本。若直接解析失败或遭遇防盗链，将自动跳转至“嗅探”功能。
      </div>
    </div>
  </Modal>

  <Modal show={showPlaylistModal} title="合集下载选择" onclose={() => showPlaylistModal = false}>
    <div class="space-y-4 flex flex-col h-[60vh]">
      <div class="flex justify-between items-end shrink-0">
        <div>
          <h4 class="text-sm font-medium text-zinc-200 line-clamp-1" title={parsedInfo?.title}>{parsedInfo?.title}</h4>
          <p class="text-xs text-zinc-500 mt-1">共 {parsedInfo?.playlist_entries?.length || 0} 个项目 · 已选 <span class="text-accent-blue">{selectedItems.size}</span> 个</p>
        </div>
        <div class="flex space-x-2">
          <button 
            class="text-xs text-zinc-400 hover:text-zinc-200 border border-zinc-700 hover:bg-zinc-800/50 px-3 py-1.5 rounded transition-colors"
            onclick={toggleSelectCurrentPage}
          >
            本页全选/反选
          </button>
          <button 
            class="text-xs text-zinc-400 hover:text-zinc-200 border border-zinc-700 hover:bg-zinc-800/50 px-3 py-1.5 rounded transition-colors"
            onclick={toggleSelectAll}
          >
            全选/反选
          </button>
        </div>
      </div>

      <div class="flex-1 overflow-y-auto border border-zinc-800 rounded-lg bg-zinc-950 p-2 space-y-1">
        {#if paginatedEntries.length > 0}
          {#each paginatedEntries as entry, i}
            {@const globalIndex = (currentPage - 1) * pageSize + i}
            {@const idx = getEntryId(entry, globalIndex)}
            <button 
              class="w-full flex items-center space-x-3 p-2 rounded hover:bg-zinc-800/50 transition-colors text-left"
              onclick={() => toggleItem(idx)}
            >
              <div class="w-4 h-4 shrink-0 rounded border {selectedItems.has(idx) ? 'bg-accent-blue border-accent-blue' : 'border-zinc-600'} flex items-center justify-center transition-colors">
                {#if selectedItems.has(idx)}
                  <svg class="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7"/></svg>
                {/if}
              </div>
              <span class="text-xs text-zinc-500 w-8 shrink-0 text-right">{idx}.</span>
              <span class="text-sm text-zinc-300 truncate flex-1" title={entry.title}>{entry.title}</span>
            </button>
          {/each}
        {/if}
      </div>

      {#if totalPages > 1}
        <div class="flex justify-between items-center shrink-0 pt-1">
          <button 
            class="px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200 bg-zinc-800/50 hover:bg-zinc-800 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            disabled={currentPage === 1}
            onclick={() => currentPage -= 1}
          >
            上一页
          </button>
          <span class="text-xs text-zinc-500 font-mono">
            {currentPage} / {totalPages}
          </span>
          <button 
            class="px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200 bg-zinc-800/50 hover:bg-zinc-800 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            disabled={currentPage === totalPages}
            onclick={() => currentPage += 1}
          >
            下一页
          </button>
        </div>
      {/if}

      <div class="shrink-0 pt-3 flex justify-end space-x-2 border-t border-zinc-800/50">
        <button 
          class="px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200 transition-colors"
          onclick={() => showPlaylistModal = false}
        >
          取消
        </button>
        <button 
          class="px-5 py-2 bg-accent-blue hover:bg-blue-600 text-white text-sm font-medium rounded-lg disabled:opacity-50 transition-colors"
          disabled={selectedItems.size === 0}
          onclick={handleCommitPlaylist}
        >
          添加至下载队列
        </button>
      </div>
    </div>
  </Modal>
</div>