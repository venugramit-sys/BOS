<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    Database,
    Zap,
    Activity,
    Layers
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
</script>

<!-- Desktop Status Bar (Hidden on Mobile < 768px) -->
<footer class="hidden md:flex h-6 border-t border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950 px-3 items-center justify-between text-[11px] text-slate-500 dark:text-slate-400 shrink-0 select-none font-mono">
  <!-- Left: Real DB & Engine Status -->
  <div class="flex items-center gap-4">
    <div class="flex items-center gap-1.5">
      <span class={cn("size-1.5 rounded-full", shell.isOnline ? "bg-emerald-500 animate-pulse" : "bg-rose-500")}></span>
      <span class="font-medium text-slate-700 dark:text-slate-300">
        Engine {shell.runtimeStatus}
      </span>
    </div>

    {#if shell.activeDatabase}
      <div class="flex items-center gap-1">
        <Database class="size-3 text-slate-400" />
        <span>db: {shell.activeDatabase}</span>
      </div>
    {/if}

    {#if shell.activeStorageStrategy}
      <div class="flex items-center gap-1 text-slate-400">
        <Layers class="size-3" />
        <span>{shell.activeStorageStrategy}</span>
      </div>
    {/if}
  </div>

  <!-- Right: Meta Objects Count & Version -->
  <div class="flex items-center gap-4">
    {#if shell.databaseSize}
      <div class="flex items-center gap-1">
        <span>size: {shell.databaseSize}</span>
      </div>
    {/if}

    <div>
      <span>v{shell.engineVersion || "0.1.0"}</span>
    </div>
  </div>
</footer>
