<script>
  /** NavigatePage wizard: one step at a time, Back / Next, required fields checked before moving on. */
  import { Check } from "lucide-svelte";
  import { Button } from "$lib/bos-ui/button/index.js";
  import { missingRequired } from "./record-model.js";
  import { nodeAttrs, isHidden } from "./node.js";

  let { p, steps = [] } = $props();

  const available = $derived(steps.filter((s) => p.design || (s.visible !== false && p.visible(s.c))));
  const index = $derived(Math.min(p.step, Math.max(available.length - 1, 0)));
  const current = $derived(available[index] ?? null);
  let message = $state("");

  $effect(() => {
    p.stepCount = available.length;
  });

  function controlsOf(step) {
    const prefix = step.c.path + "/";
    return p.C.filter((x) => x.path.startsWith(prefix));
  }

  function next() {
    const missing = missingRequired(controlsOf(current), p.rec, p.vars);
    if (missing.length) {
      message = `Fill in: ${missing.join(", ")}`;
      return;
    }
    message = "";
    p.step = Math.min(index + 1, available.length - 1);
  }

  function back() {
    message = "";
    p.step = Math.max(index - 1, 0);
  }
</script>

{#if available.length}
  <div class="bos-wizard">
    <ol class="bos-wizard-steps" aria-label="Steps">
      {#each available as s, i (s.c.i)}
        <li class={i === index ? "is-current" : i < index ? "is-done" : ""} aria-current={i === index ? "step" : undefined}>
          <span class="bos-wizard-dot">{#if i < index}<Check class="size-3" />{:else}{i + 1}{/if}</span>
          <span class="hidden sm:inline">{s.c.caption ?? s.c.name}</span>
        </li>
      {/each}
    </ol>
    {#if current}
      <section class="bos-wizard-body" {...nodeAttrs(p, current.c, isHidden(p, current.c, current.visible))}>
        <h3 class="bos-section-title">{current.c.caption ?? current.c.name}</h3>
        {#if current.c.instructional_text}<p class="bos-hint mb-3">{current.c.instructional_text}</p>{/if}
        <div class="bos-fields">{@render current.body()}</div>
      </section>
    {/if}
    {#if message}<p class="text-xs text-[var(--bos-color-unfavorable)]" role="alert">{message}</p>{/if}
    <div class="flex items-center justify-between gap-2">
      <Button variant="secondary" disabled={index === 0} onclick={back}>Back</Button>
      {#if index < available.length - 1}
        <Button onclick={next}>Next</Button>
      {/if}
    </div>
  </div>
{/if}
