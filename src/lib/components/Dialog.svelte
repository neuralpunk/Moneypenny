<!--
  AdwDialog: a floating modal with its own header bar. Uses <dialog> for focus
  trapping, Esc to close and the backdrop.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    open: boolean;
    title: string;
    width?: number;
    /** Called after the dialog closes for any reason. */
    onclose?: () => void;
    start?: Snippet;
    end?: Snippet;
    children: Snippet;
  }

  let { open = $bindable(), title, width = 420, onclose, start, end, children }: Props = $props();

  let el: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (!el) return;
    if (open && !el.open) el.showModal();
    if (!open && el.open) el.close();
  });
</script>

<dialog
  bind:this={el}
  class="adw-dialog"
  style:width="{width}px"
  aria-label={title}
  onclose={() => {
    open = false;
    onclose?.();
  }}
>
  {#if open}
    <div class="flex max-h-[calc(100vh-24px)] flex-col">
      <header class="grid h-[47px] shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-1.5 px-1.5">
        <div class="flex items-center gap-1.5">{@render start?.()}</div>
        <h2 class="truncate px-3 heading">{title}</h2>
        <div class="flex items-center justify-end gap-1.5">{@render end?.()}</div>
      </header>
      <div class="scroll min-h-0 flex-1 px-3 pt-3 pb-6 sm:px-6">
        {@render children()}
      </div>
    </div>
  {/if}
</dialog>
