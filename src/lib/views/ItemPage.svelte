<!-- Item details, pushed over the item list (AdwNavigationView-style). -->
<script lang="ts">
  import { collectionLabel, formatDate, schemaInfo, visibleAttributes } from "../format";
  import type { ItemInfo } from "../keyring";
  import { actions } from "../actions.svelte";
  import { desktop } from "../desktop.svelte";
  import { dialogs } from "../dialogs.svelte";
  import { app } from "../state.svelte";
  import ActionRow from "../components/ActionRow.svelte";
  import ButtonRow from "../components/ButtonRow.svelte";
  import Button from "../components/Button.svelte";
  import HeaderBar from "../components/HeaderBar.svelte";
  import PreferencesGroup from "../components/PreferencesGroup.svelte";
  import SecretRow from "./SecretRow.svelte";

  let { item }: { item: ItemInfo } = $props();

  const attributes = $derived(visibleAttributes(item));

  async function remove() {
    const ok = await dialogs.ask({
      heading: `Delete “${item.label || "Untitled"}”?`,
      body: "The app that saved this secret may ask you to sign in again. This can't be undone.",
      confirm: "Delete",
      destructive: true,
    });
    if (ok) await actions.deleteItem(item);
  }
</script>

<HeaderBar endControls startControls={desktop.narrow} title={item.label || "Untitled"}>
  {#snippet start()}
    <Button variant="flat" icon="go-previous" tooltip="Back" onclick={() => app.closeItem()} />
  {/snippet}
  {#snippet end()}
    <Button variant="flat" icon="document-edit" tooltip="Edit Secret" onclick={() => (dialogs.editor = { mode: "edit", item })} />
  {/snippet}
</HeaderBar>

<div class="scroll flex-1">
  <div class="mx-auto flex max-w-[600px] flex-col gap-6 px-3 pt-6 pb-20">
    <PreferencesGroup>
      <SecretRow {item} />
    </PreferencesGroup>

    <PreferencesGroup title="Details">
      <ActionRow property title="Keyring" subtitle={app.selectedCollection ? collectionLabel(app.selectedCollection) : ""} />
      <ActionRow property title="Kind" subtitle={schemaInfo(item).name} />
      {#if item.schema}
        <ActionRow property selectable monospace title="Schema" subtitle={item.schema} />
      {/if}
      <ActionRow property title="Created" subtitle={formatDate(item.created)} />
      <ActionRow property title="Modified" subtitle={formatDate(item.modified)} />
    </PreferencesGroup>

    {#if attributes.length}
      <PreferencesGroup
        title="Attributes"
        description="Apps use these to find their secrets."
      >
        {#each attributes as [key, value] (key)}
          <ActionRow property selectable title={key} subtitle={value} />
        {/each}
      </PreferencesGroup>
    {/if}

    <PreferencesGroup>
      <ButtonRow destructive label="Delete Secret…" onclick={remove} />
    </PreferencesGroup>
  </div>
</div>
