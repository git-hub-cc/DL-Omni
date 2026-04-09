<script lang="ts">
  import { configStore } from '$lib/stores/config.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let config = $derived(configStore.settings);
  let isCheckingUpdate = $state(false);
  let updateStatusText = $state('检查引擎更新');

  async function selectDirectory() {
    const selected = await open({ directory: true });
    if (selected) {
      configStore.update({ default_download_path: selected as string });
    }
  }

  async function checkUpdate() {
    isCheckingUpdate = true;
    updateStatusText = '正在获取云端版本...';
    try {
      const res = await invoke<{updated: boolean, version: string}>('check_engine_update');
      configStore.update({ yt_dlp_version: res.version });
      alert(res.updated ? `更新成功: ${res.version}` : `已是最新: ${res.version}`);
    } catch(e) {
      alert(`更新失败: ${e}`);
    } finally {
      isCheckingUpdate = false;
      updateStatusText = '检查引擎更新';
    }
  }
</script>

<div class="h-full overflow-y-auto p-6 space-y-8">
  <h2 class="text-lg font-medium text-zinc-100 tracking-wide">全局设置</h2>

  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">解析与下载</h3>
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-6">
      <div class="flex justify-between items-center">
        <div class="pr-6">
          <div class="text-sm font-medium text-zinc-200">使用内置浏览器 Cookie</div>
          <div class="text-xs text-zinc-500 mt-1">开启后将使用内置浏览器 Cookie。请先前往左侧『嗅探』页面，访问目标网站并完成登录。</div>
        </div>
        <label class="relative inline-flex items-center cursor-pointer shrink-0">
          <input type="checkbox" class="sr-only peer" checked={config.use_cookie} onchange={(e) => configStore.update({ use_cookie: e.currentTarget.checked })}>
          <div class="w-11 h-6 bg-zinc-700 rounded-full peer peer-checked:bg-accent-blue after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:after:translate-x-full"></div>
        </label>
      </div>
    </div>
  </section>

  <!-- 恢复引擎配置 -->
  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">引擎与网络控制</h3>
    
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-6">
      
      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">分开下载音频与视频轨道</div>
          <div class="text-xs text-zinc-500 mt-1">默认关闭。开启后，即使选择了最佳画质，音视频也将作为两个独立的文件分别保存（不进行合并）。</div>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input 
            type="checkbox" 
            class="sr-only peer" 
            checked={config.split_audio_video}
            onchange={(e) => configStore.update({ split_audio_video: e.currentTarget.checked })}
          >
          <div class="w-11 h-6 bg-zinc-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-accent-blue"></div>
        </label>
      </div>

      <hr class="border-zinc-800">

      <div>
        <div class="text-sm font-medium text-zinc-200 mb-3">默认视频画质</div>
        <div class="grid grid-cols-5 gap-2">
          {#each ['best', '1080p', '720p', '480p', '360p'] as q}
            <button
              class="py-1.5 text-xs font-medium rounded-lg border transition-all
                {config.video_quality === q
                  ? 'bg-accent-blue/20 border-accent-blue text-accent-blue'
                  : 'bg-zinc-800/50 border-zinc-700 text-zinc-400 hover:border-zinc-500 hover:text-zinc-200'}"
              onclick={() => configStore.update({ video_quality: q })}
            >
              {q === 'best' ? '最高' : q}
            </button>
          {/each}
        </div>
        <div class="text-xs text-zinc-600 mt-2">选择最高时引擎将自动选取可用的最佳画质</div>
      </div>

      <div>
        <div class="text-sm font-medium text-zinc-200 mb-3">默认音频音质</div>
        <div class="grid grid-cols-4 gap-2">
          {#each ['best', '320k', '128k', '64k'] as q}
            <button
              class="py-1.5 text-xs font-medium rounded-lg border transition-all
                {config.audio_quality === q
                  ? 'bg-accent-blue/20 border-accent-blue text-accent-blue'
                  : 'bg-zinc-800/50 border-zinc-700 text-zinc-400 hover:border-zinc-500 hover:text-zinc-200'}"
              onclick={() => configStore.update({ audio_quality: q })}
            >
              {q === 'best' ? '最高' : q}
            </button>
          {/each}
        </div>
      </div>
      
      <hr class="border-zinc-800">

      <div>
        <div class="flex justify-between text-sm mb-2">
          <span class="font-medium text-zinc-200">最大同时下载任务数</span>
          <span class="text-accent-blue font-mono">{config.max_concurrent_tasks}</span>
        </div>
        <input 
          type="range" min="1" max="10" 
          value={config.max_concurrent_tasks}
          oninput={(e) => configStore.update({ max_concurrent_tasks: parseInt(e.currentTarget.value) })}
          class="w-full accent-accent-blue"
        />
      </div>

      <div>
        <div class="flex justify-between text-sm mb-2">
          <span class="font-medium text-zinc-200">单任务下载并发分片数</span>
          <span class="text-accent-blue font-mono">{config.max_threads_per_task}</span>
        </div>
        <input 
          type="range" min="1" max="32" 
          value={config.max_threads_per_task}
          oninput={(e) => configStore.update({ max_threads_per_task: parseInt(e.currentTarget.value) })}
          class="w-full accent-accent-blue"
        />
      </div>
      
      <hr class="border-zinc-800">

      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">yt-dlp 核心解析引擎</div>
          <div class="text-xs text-zinc-500 mt-1">当前环境版本: {config.yt_dlp_version || '未知'}</div>
        </div>
        <button 
          class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-sm font-medium text-zinc-200 rounded-lg transition-colors border border-zinc-700/50"
          onclick={checkUpdate}
          disabled={isCheckingUpdate}
        >
          {updateStatusText}
        </button>
      </div>
    </div>
  </section>

  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">嗅探与自动命名</h3>
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-6">
      <div>
        <div class="text-sm font-medium text-zinc-200">文件命名模板</div>
        <div class="text-xs text-zinc-500 mt-1 mb-3">使用占位符自定义嗅探结果的默认名称</div>
        <input 
          type="text" 
          class="w-full bg-zinc-950 border border-zinc-700 rounded-lg px-4 py-2 text-sm text-zinc-100 outline-none focus:border-accent-blue"
          bind:value={configStore.settings.naming_template}
          onchange={() => configStore.update({ naming_template: configStore.settings.naming_template })}
        />
        <div class="flex gap-4 mt-2 text-[10px] text-zinc-500">
          <span>[title] 网页标题</span>
          <span>[name] 文件原始名</span>
          <span>[ext] 扩展名</span>
          <span>[time] 日期戳</span>
        </div>
      </div>

      <hr class="border-zinc-800">

      <div>
        <div class="text-sm font-medium text-zinc-200">嗅探正则黑名单</div>
        <div class="text-xs text-zinc-500 mt-1 mb-3">匹配该正则的 URL 将被静默忽略 (用于屏蔽广告/统计)</div>
        <input 
          type="text" 
          class="w-full bg-zinc-950 border border-zinc-700 rounded-lg px-4 py-2 text-sm font-mono text-emerald-400 outline-none focus:border-accent-blue"
          bind:value={configStore.settings.sniff_blacklist}
          onchange={() => configStore.update({ sniff_blacklist: configStore.settings.sniff_blacklist })}
        />
      </div>
    </div>
  </section>

  <section class="space-y-4">
    <h3 class="text-xs font-bold text-zinc-500 uppercase tracking-wider">文件整理与存储</h3>
    <div class="p-5 bg-zinc-800/30 border border-zinc-800 rounded-xl space-y-6">
      <div class="flex justify-between items-center">
        <div>
          <div class="text-sm font-medium text-zinc-200">默认保存位置</div>
          <div class="text-xs text-zinc-500 mt-1">{config.default_download_path || '未设置'}</div>
        </div>
        <button class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-sm font-medium text-zinc-200 rounded-lg border border-zinc-700/50" onclick={selectDirectory}>更改目录</button>
      </div>

      <hr class="border-zinc-800">

      <div class="flex justify-between items-center">
        <div class="pr-6">
          <div class="text-sm font-medium text-zinc-200">独立目录与附带元数据归档</div>
          <div class="text-xs text-zinc-500 mt-1">为每个视频创建文件夹并下载封面、字幕及 JSON 描述信息。</div>
        </div>
        <label class="relative inline-flex items-center cursor-pointer shrink-0">
          <input type="checkbox" class="sr-only peer" checked={config.include_metadata} onchange={(e) => configStore.update({ include_metadata: e.currentTarget.checked })}>
          <div class="w-11 h-6 bg-zinc-700 rounded-full peer peer-checked:bg-accent-blue after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:after:translate-x-full"></div>
        </label>
      </div>
    </div>
  </section>

</div>