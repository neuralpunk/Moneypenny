<!-- AdwToastOverlay: pill-shaped toast at the bottom centre. -->
<script lang="ts">
  import { fly } from "svelte/transition";
  import { desktop } from "../desktop.svelte";
  import { toasts } from "../toast.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center p-3" aria-live="polite">
  {#if toasts.current}
    {@const toast = toasts.current}
    {#key toast.id}
      <div
        transition:fly={{ y: 24, duration: desktop.settings.reducedMotion ? 0 : 200 }}
        class="pointer-events-auto flex min-h-[42px] max-w-[min(100%,560px)] items-center gap-2 rounded-full
               bg-[rgb(0_0_6/80%)] py-1 pr-1.5 pl-5 text-white shadow-lg"
        role="status"
      >
        <span class="min-w-0 flex-1">{toast.title}</span>
        <button
          type="button"
          class="grid size-8 place-items-center rounded-full hover:bg-white/10"
          aria-label="Dismiss"
          title="Dismiss"
          onclick={() => toasts.dismiss(toast.id)}
        >
          <Icon name="window-close" />
        </button>
      </div>
    {/key}
  {/if}
</div>
