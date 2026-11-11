<!-- Content pane: the selected keyring's items, with search. -->
<script lang="ts">
  import { collectionLabel, itemCount, itemSubtitle, schemaInfo } from "../format";
  import { actions, pending } from "../actions.svelte";
  import { desktop } from "../desktop.svelte";
  import { dialogs } from "../dialogs.svelte";
  import type { CollectionInfo } from "../keyring";
  import { app } from "../state.svelte";
  import Button from "../components/Button.svelte";
  import MenuButton, { type MenuItem } from "../components/MenuButton.svelte";
  import ActionRow from "../components/ActionRow.svelte";
  import HeaderBar from "../components/HeaderBar.svelte";
  import Icon from "../components/Icon.svelte";
  import SearchEntry from "../components/SearchEntry.svelte";
  import Spinner from "../components/Spinner.svelte";
  import StatusPage from "../components/StatusPage.svelte";
  import ErrorStatus from "./ErrorStatus.svelte";

  const collection = $derived(app.selectedCollection);
  const label = $derived(collection ? collectionLabel(collection) : "");
  const searchable = $derived(app.items.status === "ready" && app.items.data.length > 0);
  const unlocking = $derived(collection ? pending.has(`unlock:${collection.id}`) : false);

  async function deleteKeyring(c: CollectionInfo) {
    const name = collectionLabel(c);
    const ok = await dialogs.ask({
      heading: `Delete “${name}”?`,
      body:
        `This permanently deletes the keyring and its ${itemCount(c.itemCount)}. ` +
        "Apps that stored secrets in it may stop working." +
        (c.locked ? " You'll be asked to unlock it first." : ""),
      confirm: "Delete",
      destructive: true,
    });
    if (ok) await actions.deleteKeyring(c);
  }

  // The session keyring is managed by the system: no password to change and
  // nothing to gain from locking or deleting it.
  const menu = $derived.by((): MenuItem[][] => {
    const c = collection;
    if (!c || c.isSession) return [];
    return [
      [
        c.locked
          ? { label: "Unlock…", onselect: () => void actions.unlock(c) }
          : { label: "Lock", onselect: () => void actions.lock(c) },
        { label: "Change Password…", onselect: () => void actions.changePassword(c) },
      ],
      [{ label: "Delete Keyring…", destructive: true, onselect: () => void deleteKeyring(c) }],
    ];
  });
</script>

<HeaderBar endControls startControls={desktop.narrow} title={searchable ? undefined : label}>
  {#snippet start()}
    {#if desktop.narrow}
      <Button variant="flat" icon="go-previous" tooltip="Keyrings" onclick={() => app.back()} />
    {/if}
    {#if collection}
      <Button
        variant="flat"
        icon="list-add"
        tooltip="Add Secret"
        disabled={collection.locked || app.items.status !== "ready"}
        onclick={() => (dialogs.editor = { mode: "create", collectionId: collection.id })}
      />
    {/if}
  {/snippet}
  {#snippet end()}
    {#if menu.length}
      <MenuButton icon="view-more" tooltip="Keyring Menu" sections={menu} />
    {/if}
  {/snippet}
  {#snippet titleWidget()}
    {#if searchable}
      <SearchEntry
        bind:value={app.query}
        placeholder="Search {label}"
        class="w-[360px] max-w-full"
      />
    {:else}
      {label}
    {/if}
  {/snippet}
</HeaderBar>

{#if app.collections.status === "error"}
  <ErrorStatus error={app.collections.error} onretry={() => app.refresh()} />
{:else if app.collections.status === "loading" || (collection && app.items.status === "loading")}
  <div class="grid flex-1 place-items-center"><Spinner /></div>
{:else if !collection}
  <StatusPage
    icon="channel-secure"
    title="No Keyring Selected"
    description="Choose a keyring from the sidebar to see its passwords and secrets."
  />
{:else if app.items.status === "error"}
  {#if app.items.error.kind === "locked"}
    <StatusPage
      icon="system-lock-screen"
      title="Keyring Locked"
      description="Unlock “{label}” to see its {itemCount(collection.itemCount)}."
    >
      <Button pill variant="suggested" disabled={unlocking} onclick={() => actions.unlock(collection)}>
        {#if unlocking}<Spinner size={16} label="Unlocking" />{/if}
        Unlock…
      </Button>
    </StatusPage>
  {:else}
    <ErrorStatus error={app.items.error} onretry={() => app.refresh()} />
  {/if}
{:else if app.items.status === "ready" && app.items.data.length === 0}
  <StatusPage
    icon="dialog-password"
    title="No Secrets"
    description={collection.isSession
      ? "Secrets stored here by apps only last until you log out."
      : "Passwords and keys that apps save in this keyring will appear here."}
  />
{:else if app.filteredItems.length === 0}
  <StatusPage icon="system-search" title="No Results Found" description="Try a different search." />
{:else}
  <div class="scroll flex-1">
    <div class="mx-auto flex max-w-[600px] flex-col gap-3 px-3 pt-6 pb-20">
      <p class="px-0.5 text-[9pt] dim" aria-live="polite">
        {app.query ? `${app.filteredItems.length} of ${itemCount(app.items.status === "ready" ? app.items.data.length : 0)}` : itemCount(app.filteredItems.length)}
      </p>
      <div class="boxed-list" role="list">
        {#each app.filteredItems as item (item.id)}
          <ActionRow title={item.label || "Untitled"} subtitle={itemSubtitle(item)} onclick={() => app.openItem(item.id)}>
            {#snippet prefix()}<Icon name={schemaInfo(item).icon} class="dim" />{/snippet}
            {#snippet suffix()}<Icon name="go-next" class="dim" />{/snippet}
          </ActionRow>
        {/each}
      </div>
    </div>
  </div>
{/if}
