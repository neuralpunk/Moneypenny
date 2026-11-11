<!-- Hosts the app's dialogs; which one is open lives in dialogs.svelte.ts. -->
<script lang="ts">
  import { dialogs } from "../dialogs.svelte";
  import AlertDialog from "../components/AlertDialog.svelte";
  import ItemEditorDialog from "./ItemEditorDialog.svelte";
  import AboutDialog from "./AboutDialog.svelte";
  import NewKeyringDialog from "./NewKeyringDialog.svelte";
  import ShortcutsDialog from "./ShortcutsDialog.svelte";

  let confirmOpen = $state(false);
  $effect(() => {
    confirmOpen = dialogs.confirm !== null;
  });
</script>

<NewKeyringDialog />
<AboutDialog />
<ShortcutsDialog />

{#if dialogs.editor}
  {#key dialogs.editor}
    <ItemEditorDialog request={dialogs.editor} />
  {/key}
{/if}

{#if dialogs.confirm}
  {@const req = dialogs.confirm}
  <AlertDialog
    bind:open={confirmOpen}
    heading={req.heading}
    body={req.body}
    cancel="cancel"
    responses={[
      { id: "cancel", label: "Cancel" },
      { id: "confirm", label: req.confirm, appearance: req.destructive ? "destructive" : "suggested" },
    ]}
    onresponse={(id) => {
      // `req` is reactive: take the request before clearing it.
      const answered = dialogs.confirm;
      dialogs.confirm = null;
      answered?.resolve(id === "confirm");
    }}
  />
{/if}
