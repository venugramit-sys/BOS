<script>
  import { toastStore } from "./toastStore.svelte.js";
  import ToastItem from "./ToastItem.svelte";
</script>

<div class="pointer-events-none fixed bottom-4 right-4 z-[70] flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2">
  {#each toastStore.items as item (item.id)}
    <div class="pointer-events-auto">
      <ToastItem {item} />
    </div>
  {/each}
</div>
