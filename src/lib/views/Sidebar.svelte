<!-- Sidebar pane: the list of keyrings. -->
<script lang="ts">
  import { desktop } from "../desktop.svelte";
  import { dialogs } from "../dialogs.svelte";
  import { collectionLabel, collectionSubtitle } from "../format";
  import { app } from "../state.svelte";
  import Button from "../components/Button.svelte";
  import HeaderBar from "../components/HeaderBar.svelte";
  import Icon from "../components/Icon.svelte";
  import MenuButton from "../components/MenuButton.svelte";
  import Spinner from "../components/Spinner.svelte";

  // Up/Down move between keyrings, like a GTK list.
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>("button")];
    const i = rows.indexOf(document.activeElement as HTMLButtonElement);
    const next = rows[Math.min(Math.max(i + (e.key === "ArrowDown" ? 1 : -1), 0), rows.length - 1)];
    next?.focus();
    e.preventDefault();
  }
</script>

<aside
  class="flex shrink-0 flex-col {desktop.narrow ? 'flex-1' : 'w-[clamp(200px,28%,280px)]'} text-sidebar-fg shadow-[inset_-1px_0_var(--sidebar-shade-color)] transition-colors duration-200"
  class:bg-sidebar={desktop.focused}
  class:bg-sidebar-backdrop={!desktop.focused}
>
  <HeaderBar title="Keyrings" startControls endControls={desktop.narrow}>
    {#snippet start()}
      <Button
        variant="flat"
        icon="list-add"
        tooltip="New Keyring"
        disabled={app.collections.status !== "ready"}
        onclick={() => (dialogs.newKeyring = true)}
      />
    {/snippet}
    {#snippet end()}
      <Button
        variant="flat"
        icon="view-refresh"
        tooltip="Reload"
        disabled={app.refreshing}
        onclick={() => app.refresh()}
      />
      <MenuButton
        tooltip="Main Menu"
        sections={[
          [
            { label: "Keyboard Shortcuts", onselect: () => (dialogs.shortcuts = true) },
            { label: "About Moneypenny", onselect: () => (dialogs.about = true) },
          ],
        ]}
      />
    {/snippet}
  </HeaderBar>

  <nav class="scroll flex-1 px-1.5 pb-1.5" aria-label="Keyrings">
    {#if app.collections.status === "loading"}
      <div class="grid place-items-center py-6"><Spinner size={24} /></div>
    {:else if app.collections.status === "error"}
      <p class="px-3 py-2 text-[9pt] dim">Keyrings unavailable</p>
    {:else}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <ul class="flex flex-col gap-0.5" {onkeydown}>
        {#each app.collections.data as c (c.id)}
          <li>
            <button
              type="button"
              class="sidebar-row flex w-full items-center gap-3 px-3 py-2 text-left"
              aria-current={!desktop.narrow && c.id === app.selectedCollectionId}
              onclick={() => app.selectCollection(c.id)}
            >
              <Icon name="dialog-password" class="dim" />
              <span class="flex min-w-0 flex-1 flex-col">
                <span class="row-title truncate">{collectionLabel(c)}</span>
                <span class="truncate text-[9pt] dim">{collectionSubtitle(c)}</span>
              </span>
              {#if c.locked}
                <span title="Locked"><Icon name="changes-prevent" /></span>
                <span class="sr-only">Locked</span>
              {/if}
              {#if desktop.narrow}<Icon name="go-next" class="dim" />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </nav>
</aside>
