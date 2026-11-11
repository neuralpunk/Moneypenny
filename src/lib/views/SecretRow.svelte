<!--
  The secret itself: hidden by default, fetched only when revealed, and
  forgotten again when hidden or when leaving the page. Copying happens in
  Rust so the value doesn't need to reach the page at all.
-->
<script lang="ts">
  import { actions } from "../actions.svelte";
  import { keyring, toKeyringError, type ItemInfo, type SecretValue } from "../keyring";
  import { app } from "../state.svelte";
  import { toasts } from "../toast.svelte";
  import Button from "../components/Button.svelte";
  import Spinner from "../components/Spinner.svelte";

  let { item }: { item: ItemInfo } = $props();

  let revealed = $state<SecretValue | null>(null);
  let busy = $state(false);

  // Forget the value whenever the item changes (or is edited) or the row goes away.
  $effect(() => {
    void item.id;
    void item.modified;
    return () => {
      revealed = null;
    };
  });

  function fail(e: unknown) {
    const err = toKeyringError(e);
    if (err.kind === "locked" || err.kind === "not-found") void app.refresh();
    toasts.show(err.kind === "locked" ? "The keyring is locked" : "message" in err ? err.message : "Couldn't read the secret");
  }

  async function toggle() {
    if (revealed) {
      revealed = null;
      return;
    }
    busy = true;
    try {
      revealed = await keyring.getSecret(item.collectionId, item.id);
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

</script>

<div class="flex min-h-[50px] items-center gap-3 px-3">
  <div class="flex min-w-0 flex-1 flex-col justify-center py-2">
    <span class="text-[9pt] dim">Secret</span>
    {#if revealed?.type === "text"}
      <span class="monospace select-text break-all whitespace-pre-wrap">{revealed.value}</span>
    {:else if revealed?.type === "binary"}
      <span class="dim">Binary data ({revealed.size} bytes)</span>
    {:else}
      <span aria-label="Hidden" class="tracking-[0.2em]">●●●●●●●●●●</span>
    {/if}
  </div>
  {#if busy}<Spinner size={16} />{/if}
  <Button
    variant="flat"
    circular
    icon={revealed ? "view-conceal" : "view-reveal"}
    tooltip={revealed ? "Hide Secret" : "Show Secret"}
    aria-pressed={!!revealed}
    disabled={busy}
    onclick={toggle}
  />
  {#if revealed?.type !== "binary"}
    <Button variant="flat" circular icon="edit-copy" tooltip="Copy Secret" onclick={() => actions.copySecret(item)} />
  {/if}
</div>
