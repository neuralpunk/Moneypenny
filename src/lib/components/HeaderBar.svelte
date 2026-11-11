<!--
  libadwaita-style header bar. The bar and its title are drag regions; double
  click toggles maximize (handled by Tauri). Window controls appear on the
  sides requested via `startControls` / `endControls`, the way
  AdwNavigationSplitView splits them between sidebar and content.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { desktop } from "../desktop.svelte";
  import WindowControls from "./WindowControls.svelte";

  interface Props {
    title?: string;
    titleWidget?: Snippet;
    start?: Snippet;
    end?: Snippet;
    startControls?: boolean;
    endControls?: boolean;
    class?: string;
  }

  let {
    title,
    titleWidget,
    start,
    end,
    startControls = false,
    endControls = false,
    class: className = "",
  }: Props = $props();
</script>

<header
  data-tauri-drag-region
  class="grid h-[47px] shrink-0 shadow-[inset_0_-1px_var(--headerbar-trim)] grid-cols-[minmax(max-content,1fr)_minmax(0,auto)_minmax(max-content,1fr)] items-center gap-1.5 px-1.5 {className}"
>
  <div
    data-tauri-drag-region
    class="flex items-center gap-1.5 transition-opacity duration-200"
    class:opacity-50={!desktop.focused}
  >
    {#if startControls}<WindowControls side="start" />{/if}
    {@render start?.()}
  </div>

  <div
    data-tauri-drag-region
    class="min-w-0 truncate px-3 text-center heading transition-opacity duration-200"
    class:opacity-50={!desktop.focused}
  >
    {#if titleWidget}{@render titleWidget()}{:else if title}{title}{/if}
  </div>

  <div
    data-tauri-drag-region
    class="flex items-center justify-end gap-1.5 transition-opacity duration-200"
    class:opacity-50={!desktop.focused}
  >
    {@render end?.()}
    {#if endControls}<WindowControls side="end" />{/if}
  </div>
</header>
