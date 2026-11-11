<script lang="ts">
  import { actions } from "../actions.svelte";
  import { dialogs } from "../dialogs.svelte";
  import Button from "../components/Button.svelte";
  import Dialog from "../components/Dialog.svelte";
  import EntryRow from "../components/EntryRow.svelte";
  import PreferencesGroup from "../components/PreferencesGroup.svelte";

  let name = $state("");
  const valid = $derived(name.trim().length > 0);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!valid) return;
    const label = name;
    dialogs.newKeyring = false;
    // The system then asks for the new keyring's password.
    void actions.createKeyring(label);
  }
</script>

<Dialog bind:open={dialogs.newKeyring} title="New Keyring" width={400} onclose={() => (name = "")}>
  {#snippet start()}
    <Button label="Cancel" onclick={() => (dialogs.newKeyring = false)} />
  {/snippet}
  {#snippet end()}
    <Button variant="suggested" label="Create" type="submit" form="new-keyring" disabled={!valid} />
  {/snippet}
  <form id="new-keyring" onsubmit={submit} class="flex flex-col gap-3">
    <PreferencesGroup description="Next, you'll be asked to choose a password for it.">
      <EntryRow title="Name" bind:value={name} autofocus maxlength={128} />
    </PreferencesGroup>
  </form>
</Dialog>
