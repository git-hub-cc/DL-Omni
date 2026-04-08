<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { IPC } from '$lib/api/ipc';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { goto } from '$app/navigation';
  import type { SniffedResource } from '$lib/types';

  let url = $state('https://m.bilibili.com');
  let capturedResources = $state<SniffedResource[]>([]);
  let showDrawer = $state(false);
  let isSniffing = $state(false);
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    unlisten = await IPC.listenSniffedResources((resource: SniffedResource) => {
      // 避免重复链接
      if (!capturedResources.find(r => r.url === resource.url)) {
        capturedResources = [...capturedResources, resource];
      }
    });
  });

  onDestroy(async () => {
    if (unlisten) unlisten();
    if (isSniffing) {
      await IPC.stopSniffing();
    }
  });

  async function start() {
    if (!url) return;
    
    isSniffing = true;
    capturedResources = []; 
    showDrawer = false;
    
    try {
      await IPC.startSniffing(url);
    } catch (e) {
      console.error("嗅探启动失败:", e);
      isSniffing = false;
    }
  }

  async function stop() {
    isSniffing = false;
    try {
      await IPC.stopSniffing();
    } catch (e) {
      console.error("停止嗅探失败:", e);
    }
  }

  function handleDownload(resource: SniffedResource) {
    console.log("准备下载:", resource);
    showDrawer = false; 
    
    // 【修改】直接调用专门为嗅探准备的提交通道，不再绕路去解析
    taskStore.submitSniffedTask(resource); 
    
    goto('/'); // 路由跳转回主页任务列表
  }
</script>

<div class="h-full flex flex-col relative bg-zinc-950">
  <header class="shrink-0 p-3 flex items-center space-x-2 border-b border-zinc-800/50 bg-zinc-900">
    <div class="flex-1 relative">
      <input 
        type="text" 
        bind:value={url}
        placeholder="输入流媒体网页地址 (回车开始嗅探)"
        class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-4 py-2 text-sm text-zinc-200 outline-none focus:border-accent-blue transition-colors"
        onkeydown={(e) => e.key === 'Enter' && start()}
      />
    </div>
    {#if isSniffing}
      <button 
        class="px-4 py-2 bg-red-500/10 hover:bg-red-500/20 text-red-500 text-sm font-medium rounded-lg transition-colors"
        onclick={stop}
      >
        停止侦听
      </button>
    {:else}
      <button 
        class="px-4 py-2 bg-accent-blue hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors"
        onclick={start}
      >
        开启高级嗅探窗
      </button>
    {/if}
  </header>

  <div class="flex-1 relative flex flex-col items-center justify-center p-6">
    {#if !isSniffing}
      <svg class="w-16 h-16 text-zinc-700 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
      <h3 class="text-lg font-medium text-zinc-300 mb-2">等待输入网页</h3>
      <p class="text-sm text-zinc-500 text-center max-w-sm">采用多层级引擎。将在独立的不可见/可见窗口中渲染网页，并深度拦截底层网络 API，突破常规防盗链。</p>
    {:else}
      <div class="relative w-24 h-24 mb-6 flex items-center justify-center">
        <div class="absolute inset-0 border-4 border-zinc-800 rounded-full"></div>
        <div class="absolute inset-0 border-4 border-accent-blue rounded-full border-t-transparent animate-spin"></div>
        <svg class="w-8 h-8 text-accent-blue" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
      </div>
      <h3 class="text-lg font-medium text-zinc-200 mb-2">底层拦截引擎运行中</h3>
      <p class="text-sm text-zinc-500 text-center max-w-sm">请在弹出的窗口中操作并播放目标视频...<br/>如有验证码请手动通过。</p>
    {/if}
  </div>

  <button 
    class="absolute right-8 bottom-8 w-14 h-14 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-full shadow-xl flex items-center justify-center transition-transform hover:scale-105 group z-50"
    aria-label="查看捕获的资源"
    title="查看捕获的资源"
    onclick={() => showDrawer = !showDrawer}
  >
    <svg class="w-6 h-6 text-zinc-300" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 002-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/></svg>
    {#if capturedResources.length > 0}
      <span class="absolute top-0 right-0 -mt-1 -mr-1 flex h-5 w-5">
        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-accent-blue opacity-75"></span>
        <span class="relative inline-flex rounded-full h-5 w-5 bg-accent-blue items-center justify-center text-[10px] font-bold text-white shadow">
          {capturedResources.length}
        </span>
      </span>
    {/if}
  </button>

  {#if showDrawer}
    <div 
      class="absolute inset-0 bg-black/60 backdrop-blur-sm z-40"
      role="button"
      tabindex="0"
      onclick={() => showDrawer = false}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') showDrawer = false; }}
    ></div>
    <div class="absolute inset-x-0 bottom-0 h-[30rem] bg-zinc-900 border-t border-zinc-700 shadow-2xl flex flex-col z-50">
      <div class="flex justify-between items-center p-4 border-b border-zinc-800/50 bg-zinc-900/50">
        <h3 class="text-sm font-medium text-zinc-100 flex items-center space-x-2">
          <span>嗅探到的媒体资源</span>
          <span class="px-2 py-0.5 bg-zinc-800 border border-zinc-700 text-zinc-300 rounded-full text-xs">{capturedResources.length}</span>
        </h3>
        <button 
          aria-label="关闭抽屉"
          title="关闭抽屉"
          onclick={() => showDrawer = false} 
          class="text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
        </button>
      </div>
      <div class="flex-1 overflow-y-auto p-4 space-y-3">
        {#if capturedResources.length === 0}
          <div class="h-full flex flex-col items-center justify-center text-sm text-zinc-500 space-y-2">
            <svg class="w-8 h-8 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
            <span>尝试在独立窗口中播放视频以捕获直链</span>
          </div>
        {:else}
          {#each capturedResources as res, i}
            <div class="flex flex-col p-3 bg-zinc-800/30 border border-zinc-800 rounded-lg hover:bg-zinc-800/60 transition-colors">
              <div class="flex items-start justify-between">
                <div class="flex-1 min-w-0 pr-4">
                  <h4 class="text-xs font-medium text-zinc-200 truncate">{res.filename}</h4>
                  <p class="text-[10px] text-zinc-500 mt-1 truncate font-mono" title={res.url}>{res.url}</p>
                </div>
                <button 
                  class="shrink-0 px-3 py-1.5 bg-zinc-200 hover:bg-white text-zinc-900 text-xs font-medium rounded transition-colors shadow-sm"
                  onclick={() => handleDownload(res)}
                >
                  提取下载
                </button>
              </div>
              <div class="flex flex-wrap gap-2 mt-3">
                <span class="px-1.5 py-0.5 bg-zinc-700/50 text-accent-blue border border-zinc-700 rounded text-[9px] uppercase">
                  {res.type}
                </span>
                {#if res.headers && Object.keys(res.headers).length > 0}
                  <span class="px-1.5 py-0.5 bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 rounded text-[9px]">
                    已附带防盗链 Headers
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>