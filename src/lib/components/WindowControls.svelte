<!-- GNOME window buttons, placed according to org.gnome.desktop.wm.preferences button-layout. -->
<script lang="ts">
  import { desktop, windowActions, type WindowButton } from "../desktop.svelte";
  import Icon from "./Icon.svelte";

  let { side }: { side: "start" | "end" } = $props();

  const buttons = $derived(desktop.buttons[side]);

  const labels: Record<WindowButton, string> = {
    minimize: "Minimize",
    maximize: "Maximize",
    close: "Close",
  };

  function icon(b: WindowButton) {
    if (b === "maximize") return desktop.maximized ? "window-restore" : "window-maximize";
    return `window-${b}`;
  }
</script>

{#if buttons.length}
  <div class="flex items-center gap-0.5" class:order-first={side === "start"}>
    {#each buttons as b (b)}
      <button
        type="button"
        class="group grid size-[34px] place-items-center rounded-full outline-none"
        aria-label={desktop.maximized && b === "maximize" ? "Restore" : labels[b]}
        title={desktop.maximized && b === "maximize" ? "Restore" : labels[b]}
        onclick={() => windowActions[b]()}
      >
        <span
          class="grid size-6 place-items-center rounded-full bg-current/10 transition-colors
                 group-hover:bg-current/15 group-active:bg-current/30
                 group-focus-visible:outline-2 group-focus-visible:outline-accent/50"
        >
          <Icon name={icon(b)} />
        </span>
      </button>
    {/each}
  </div>
{/if}
