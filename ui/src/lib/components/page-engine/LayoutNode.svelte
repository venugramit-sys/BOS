<script>
  /**
   * LayoutNode.svelte (Svelte 5 Runes)
   * Recursive container and smart field renderer for BOS Page and PageExtension AST.
   * Renders arbitrary layout containers (grid, flex, split, tabs, card, accordion)
   * and binds fields to their interconnected meta-object definitions.
   */
  import { cn } from "$lib/utils.js";
  import {
    Field,
    Input,
    DatePicker,
    Combobox,
    Select,
    Switch,
    SplitPane,
    Tabs,
    Badge,
    Button
  } from "$lib/bos-ui/index.js";
  import LayoutNode from "./LayoutNode.svelte";

  let {
    node,
    record = $bindable({}),
    resolvedFields = {},
    mode = "edit",
    fieldErrors = {},
    onFieldChange = null,
    onAction = null
  } = $props();

  // Normalize container kind
  let kind = $derived(node?.kind?.toLowerCase() || "field");
  let name = $derived(node?.name || "");
  let caption = $derived(node?.caption || resolvedFields[name]?.caption || name);
  let children = $derived(node?.children || []);
  let properties = $derived(node?.properties || {});

  // Field metadata
  let fieldMeta = $derived(resolvedFields[name] || {});
  let dataType = $derived(fieldMeta?.data_type?.toLowerCase() || "string");
  let isReadOnly = $derived(
    mode === "view" ||
    properties?.readonly === "true" ||
    properties?.editable === "false" ||
    fieldMeta?.editable === false ||
    fieldMeta?.is_flow_field === true
  );
  let isRequired = $derived(
    properties?.required === "true" ||
    fieldMeta?.not_null === true
  );
  let isVisible = $derived(properties?.visible !== "false");

  // Grid column class computation
  let gridColsClass = $derived.by(() => {
    const cols = parseInt(properties?.cols || "1", 10);
    switch (cols) {
      case 2: return "grid grid-cols-1 md:grid-cols-2";
      case 3: return "grid grid-cols-1 md:grid-cols-3";
      case 4: return "grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4";
      case 6: return "grid grid-cols-2 md:grid-cols-6";
      default: return "grid grid-cols-1";
    }
  });

  // Gap computation
  let gapClass = $derived.by(() => {
    const gap = parseInt(properties?.gap || "3", 10);
    switch (gap) {
      case 1: return "gap-1";
      case 2: return "gap-2";
      case 4: return "gap-4";
      case 6: return "gap-6";
      default: return "gap-3";
    }
  });

  function handleInput(val) {
    record[name] = val;
    onFieldChange?.(name, val);
  }
</script>

{#if isVisible}
  {#if kind === "grid"}
    <div class={cn(gridColsClass, gapClass, "w-full min-w-0")}>
      {#each children as child (child.name || child.kind + Math.random())}
        <LayoutNode
          node={child}
          bind:record
          {resolvedFields}
          {mode}
          {fieldErrors}
          {onFieldChange}
          {onAction}
        />
      {/each}
    </div>

  {:else if kind === "flex"}
    <div
      class={cn(
        "flex w-full min-w-0 flex-wrap",
        properties?.direction === "column" ? "flex-col" : "flex-row",
        properties?.align === "center" ? "items-center" : "items-start",
        gapClass
      )}
    >
      {#each children as child (child.name || child.kind + Math.random())}
        <LayoutNode
          node={child}
          bind:record
          {resolvedFields}
          {mode}
          {fieldErrors}
          {onFieldChange}
          {onAction}
        />
      {/each}
    </div>

  {:else if kind === "card" || kind === "group" || kind === "section" || kind === "panel"}
    <div class="rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/50 p-4 shadow-sm w-full min-w-0">
      {#if caption}
        <div class="flex items-center justify-between pb-3 mb-3 border-b border-slate-100 dark:border-slate-800/60">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-700 dark:text-slate-300">
            {caption}
          </h3>
          {#if properties?.badge}
            <Badge variant="outline">{properties.badge}</Badge>
          {/if}
        </div>
      {/if}
      <div class="space-y-3">
        {#each children as child (child.name || child.kind + Math.random())}
          <LayoutNode
            node={child}
            bind:record
            {resolvedFields}
            {mode}
            {fieldErrors}
            {onFieldChange}
            {onAction}
          />
        {/each}
      </div>
    </div>

  {:else if kind === "split"}
    {@const primaryChild = children[0]}
    {@const secondaryChild = children[1]}
    <div class="w-full min-h-[300px]">
      <SplitPane direction={properties?.orientation === "vertical" ? "column" : "row"}>
        {#snippet primary()}
          {#if primaryChild}
            <LayoutNode
              node={primaryChild}
              bind:record
              {resolvedFields}
              {mode}
              {fieldErrors}
              {onFieldChange}
              {onAction}
            />
          {/if}
        {/snippet}
        {#snippet secondary()}
          {#if secondaryChild}
            <LayoutNode
              node={secondaryChild}
              bind:record
              {resolvedFields}
              {mode}
              {fieldErrors}
              {onFieldChange}
              {onAction}
            />
          {/if}
        {/snippet}
      </SplitPane>
    </div>

  {:else if kind === "tabs"}
    <Tabs
      tabs={children.map((c) => ({
        id: c.name,
        label: c.caption || c.name,
        dirty: false
      }))}
      activeTabId={children[0]?.name}
    >
      {#snippet tabContent(tab)}
        {@const activeChild = children.find((c) => c.name === tab.id)}
        {#if activeChild}
          <div class="p-3">
            {#each activeChild.children as grandChild (grandChild.name || grandChild.kind + Math.random())}
              <LayoutNode
                node={grandChild}
                bind:record
                {resolvedFields}
                {mode}
                {fieldErrors}
                {onFieldChange}
                {onAction}
              />
            {/each}
          </div>
        {/if}
      {/snippet}
    </Tabs>

  {:else if kind === "field"}
    <Field
      label={caption}
      required={isRequired}
      error={fieldErrors[name] || ""}
      hint={properties?.tooltip || ""}
    >
      {#if fieldMeta?.relation_table}
        <!-- Relational lookup with Combobox -->
        <Combobox
          value={record[name]}
          disabled={isReadOnly}
          placeholder={`Select ${caption}...`}
          onChange={(val) => handleInput(val)}
        />
      {:else if dataType.includes("decimal") || dataType.includes("integer") || dataType.includes("numeric")}
        <!-- Numeric / Financial Input with built-in safe arithmetic evaluator -->
        <Input
          value={record[name] ?? ""}
          disabled={isReadOnly}
          align="right"
          placeholder="0.00"
          oninput={(e) => handleInput(e.target.value)}
        />
      {:else if dataType.includes("date")}
        <!-- Date field with NAV/BC date formula shorthand engine -->
        <DatePicker
          value={record[name] ?? ""}
          disabled={isReadOnly}
          onChange={(val) => handleInput(val)}
        />
      {:else if dataType.includes("bool")}
        <!-- Boolean Switch -->
        <Switch
          checked={Boolean(record[name])}
          disabled={isReadOnly}
          onChange={(val) => handleInput(val)}
        />
      {:else if dataType.includes("enum")}
        <!-- Enum Dropdown -->
        <Select
          value={record[name]}
          disabled={isReadOnly}
          placeholder={`Choose ${caption}...`}
          onChange={(val) => handleInput(val)}
        />
      {:else}
        <!-- Default Text Input -->
        <Input
          value={record[name] ?? ""}
          disabled={isReadOnly}
          placeholder={properties?.placeholder || `Enter ${caption}...`}
          oninput={(e) => handleInput(e.target.value)}
        />
      {/if}
    </Field>
  {/if}
{/if}
