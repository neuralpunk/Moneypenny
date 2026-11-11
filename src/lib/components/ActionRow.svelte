<!--
  AdwActionRow. With `property`, the title is small and dim and the subtitle
  is the main value (libadwaita's "property" style). With `onclick`, the row
  is activatable.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    subtitle?: string;
    property?: boolean;
    /** Lets the user select the subtitle text (for values). */
    selectable?: boolean;
    monospace?: boolean;
    onclick?: () => void;
    prefix?: Snippet;
    suffix?: Snippet;
    children?: Snippet;
  }

  let {
    title,
    subtitle,
    property = false,
    selectable = false,
    monospace = false,
    onclick,
    prefix,
    suffix,
    children,
  }: Props = $props();
</script>

{#snippet body()}
  {@render prefix?.()}
  <div class="flex min-w-0 flex-1 flex-col justify-center py-2">
    {#if property}
      <span class="text-[9pt] dim">{title}</span>
      {#if subtitle !== undefined}
        <span
          class="break-words [overflow-wrap:anywhere]"
          class:select-text={selectable}
          class:monospace
        >{subtitle}</span>
      {/if}
    {:else}
      <span class="truncate">{title}</span>
      {#if subtitle}
        <span class="truncate text-[9pt] dim" class:select-text={selectable} class:monospace>{subtitle}</span>
      {/if}
    {/if}
    {@render children?.()}
  </div>
  {@render suffix?.()}
{/snippet}

{#if onclick}
  <button
    type="button"
    class="activatable flex min-h-[50px] w-full items-center gap-3 px-3 text-left"
    {onclick}
  >
    {@render body()}
  </button>
{:else}
  <div class="flex min-h-[50px] items-center gap-3 px-3">
    {@render body()}
  </div>
{/if}
