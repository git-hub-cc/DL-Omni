<script lang="ts">
  let { 
    progress = 0,    // 0.0 到 1.0 的浮点数
    speedText = "",  // 如 "15.2 MB/s"
    etaText = "",    // 如 "00:15"
    sizeText = "",   // 如 "105 MB / 1.2 GB"
    status = "downloading", 
    errorMsg = undefined // 接收后端传递的错误原因
  } = $props<{
    progress: number;
    speedText?: string;
    etaText?: string;
    sizeText?: string;
    status?: import('$lib/types').TaskStatus;
    errorMsg?: string;
  }>();

  // 根据状态派生出不同的底色类名
  let colorClass = $derived(
    status === 'error' ? 'bg-red-500/80' : 
    status === 'paused' ? 'bg-zinc-500/80' : 
    status === 'completed' ? 'bg-emerald-500/80' : 
    'bg-blue-500/80'
  );
</script>

<div class="relative w-full h-5 rounded overflow-hidden bg-zinc-800/50 border border-zinc-700/30">
  
  <div 
    class="absolute top-0 left-0 h-full w-full origin-left transition-transform duration-150 ease-out {colorClass}"
    style="transform: scaleX({progress});"
  ></div>

  <div class="absolute inset-0 flex items-center justify-between px-2 text-[11px] font-mono text-zinc-100 drop-shadow-md z-10 pointer-events-none tracking-wide">
    {#if status === 'error'}
      <div class="w-full text-center sm:text-left text-red-50 font-medium truncate drop-shadow-lg px-1">
        ⚠️ {errorMsg || "下载发生异常，请重试"}
      </div>
    {:else}
      <div class="flex items-center space-x-3">
        <span>{speedText}</span>
        {#if etaText}
          <span class="text-zinc-300">ETA: {etaText}</span>
        {/if}
      </div>
      <div class="text-right">
        <span>{sizeText}</span>
        <span class="ml-1 opacity-80">({(progress * 100).toFixed(1)}%)</span>
      </div>
    {/if}
  </div>
</div>