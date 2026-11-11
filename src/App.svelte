<script lang="ts">
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { desktop } from "./lib/desktop.svelte";
  import { handleShortcut } from "./lib/shortcuts";
  import { app } from "./lib/state.svelte";
  import ToastOverlay from "./lib/components/ToastOverlay.svelte";
  import Dialogs from "./lib/views/Dialogs.svelte";
  import ItemPage from "./lib/views/ItemPage.svelte";
  import ItemsPage from "./lib/views/ItemsPage.svelte";
  import Sidebar from "./lib/views/Sidebar.svelte";

  onMount(() => {
    void app.refresh();
    if (!isTauri()) return;
    // Changes made by other apps (or the daemon restarting) arrive here.
    const unlisten = listen("keyring-changed", () => void app.refresh());
    return () => void unlisten.then((f) => f());
  });

  // Narrow windows show one pane at a time, like a collapsed AdwNavigationSplitView.
  const showSidebar = $derived(!desktop.narrow || !app.showContent);
  const showMain = $derived(!desktop.narrow || app.showContent);

  const slide = $derived({ x: 48, duration: desktop.settings.reducedMotion ? 0 : 200 });
</script>

<!-- AdwNavigationSplitView: sidebar pane + content pane. -->
<svelte:window onkeydown={handleShortcut} />

<div class="flex h-full">
  {#if showSidebar}<Sidebar />{/if}

  {#if showMain}
  <main class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-window text-window-fg">
    {#if app.selectedItem}
      {#key app.selectedItem.id}
        <div class="flex min-h-0 flex-1 flex-col" in:fly={slide}>
          <ItemPage item={app.selectedItem} />
        </div>
      {/key}
    {:else}
      <div class="flex min-h-0 flex-1 flex-col">
        <ItemsPage />
      </div>
    {/if}
    <ToastOverlay />
  </main>
  {/if}
</div>

<Dialogs />
