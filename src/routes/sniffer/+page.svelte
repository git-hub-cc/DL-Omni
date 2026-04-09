<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { IPC } from '$lib/api/ipc';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { taskStore } from '$lib/stores/tasks.svelte';
  import { goto } from '$app/navigation';
  import type { SniffedResource } from '$lib/types';

  let url = $state('https://www.douyin.com/jingxuan');
  let capturedResources = $state<SniffedResource[]>([]);
  let showDrawer = $state(false);
  let isSniffing = $state(false);
  let unlisten: UnlistenFn | null = null;

  // 快捷导航数据
  const shortcuts = [
    {
      title: '音乐检索',
      url: 'https://music.gdstudio.xyz/',
      desc: 'music.gdstudio.xyz',
      icon: 'M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3'
    },
    {
      title: '视频搜索',
      url: 'https://www.iyf.lv/',
      desc: 'www.iyf.lv',
      icon: 'M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z'
    },
    {
      title: '网盘搜索',
      url: 'https://cse.google.com/cse?cx=e7dbb37893b8e4dbf',
      desc: 'Google CSE 聚合',
      icon: 'M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z'
    }
  ];

  onMount(async () => {
    unlisten = await IPC.listenSniffedResources((resource: SniffedResource) => {
      // 避免重复链接
      if (!capturedResources.find(r => r.url === resource.url)) {
        // 修改点：将最新捕获的资源插入到数组最前面，实现倒序显示
        capturedResources = [resource, ...capturedResources];
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

    // 提交嗅探任务
    taskStore.submitSniffedTask(resource);

    goto('/'); // 路由跳转回主页任务列表
  }

  // 快捷导航点击处理
  function handleShortcutClick(targetUrl: string) {
    url = targetUrl;
    start();
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

  <div class="flex-1 relative flex flex-col items-center justify-center p-6 overflow-y-auto">
    {#if !isSniffing}
      <div class="flex flex-col items-center justify-center mt-[-10vh]">
        <svg class="w-16 h-16 text-zinc-700 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
        <h3 class="text-lg font-medium text-zinc-300 mb-2">等待输入网页</h3>
        <p class="text-sm text-zinc-500 text-center max-w-sm mb-12">采用多层级引擎。将在独立的不可见/可见窗口中渲染网页，并深度拦截底层网络 API，突破常规防盗链。</p>

        <div class="w-full max-w-2xl px-4">
          <div class="flex items-center justify-center mb-6 space-x-3">
            <div class="h-px w-12 bg-zinc-800"></div>
            <h4 class="text-xs font-bold text-zinc-500 uppercase tracking-widest">常用检索站点</h4>
            <div class="h-px w-12 bg-zinc-800"></div>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
            {#each shortcuts as item}
              <button
                class="group flex flex-col items-center p-5 bg-zinc-800/20 border border-zinc-800 rounded-xl hover:bg-zinc-800/50 hover:border-zinc-600 hover:-translate-y-0.5 transition-all duration-200"
                onclick={() => handleShortcutClick(item.url)}
              >
                <div class="w-12 h-12 rounded-full bg-zinc-800 flex items-center justify-center mb-3 group-hover:bg-accent-blue/10 transition-colors">
                  <svg class="w-6 h-6 text-zinc-400 group-hover:text-accent-blue transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d={item.icon} />
                  </svg>
                </div>
                <span class="text-sm font-medium text-zinc-200">{item.title}</span>
                <span class="text-[10px] text-zinc-500 mt-1 truncate w-full text-center font-mono opacity-70">{item.desc}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>
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
    class="absolute right-8 bottom-8 w-14 h-14 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-full shadow-xl flex items-center justify-center transition-transform hover:scale-105 group z-40"
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