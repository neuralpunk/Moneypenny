<!-- AdwStatusPage: large dimmed icon, title and description, centred. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    icon?: string;
    title: string;
    description?: string;
    compact?: boolean;
    children?: Snippet;
  }

  let { icon, title, description, compact = false, children }: Props = $props();
</script>

<div class="flex flex-1 flex-col items-center justify-center overflow-auto px-3 py-6 text-center">
  <div class="flex max-w-[360px] flex-col items-center">
    {#if icon}
      <Icon name={icon} size={compact ? 64 : 128} class="dim {compact ? 'mb-4' : 'mb-9'}" />
    {/if}
    <h1 class={compact ? "title-2" : "title-1"}>{title}</h1>
    {#if description}
      <p class="mt-3 text-balance">{description}</p>
    {/if}
    {#if children}
      <div class="mt-9">{@render children()}</div>
    {/if}
  </div>
</div>
