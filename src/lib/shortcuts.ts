// App-wide keyboard shortcuts, following GNOME conventions. The same table
// feeds the Keyboard Shortcuts dialog.

import { tick } from "svelte";
import { actions } from "./actions.svelte";
import { desktop, windowActions } from "./desktop.svelte";
import { dialogs } from "./dialogs.svelte";
import { app } from "./state.svelte";

export interface ShortcutGroup {
  title: string;
  /** `keys` lists alternatives; each is a combination of keycaps. */
  items: { title: string; keys: string[][] }[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    title: "General",
    items: [
      { title: "Search", keys: [["Ctrl", "F"]] },
      { title: "Reload", keys: [["Ctrl", "R"], ["F5"]] },
      { title: "Go Back", keys: [["Esc"], ["Alt", "←"]] },
      { title: "Keyboard Shortcuts", keys: [["Ctrl", "?"]] },
      { title: "Quit", keys: [["Ctrl", "Q"]] },
    ],
  },
  {
    title: "Keyrings",
    items: [{ title: "New Keyring", keys: [["Ctrl", "Shift", "N"]] }],
  },
  {
    title: "Secrets",
    items: [
      { title: "New Secret", keys: [["Ctrl", "N"]] },
      { title: "Copy Secret", keys: [["Ctrl", "C"]] },
      { title: "Edit Secret", keys: [["Ctrl", "E"]] },
    ],
  },
];

async function focusSearch() {
  app.closeItem();
  if (desktop.narrow) app.showContent = true;
  await tick();
  document.querySelector<HTMLInputElement>("main input[type=search]")?.focus();
}

function canAddItem() {
  const c = app.selectedCollection;
  return c !== null && !c.locked && app.items.status === "ready";
}

/** Handles a window keydown. Dialogs and menus keep their own keys. */
export function handleShortcut(e: KeyboardEvent) {
  if (e.defaultPrevented || document.querySelector("dialog[open], [popover]:popover-open")) return;
  const target = e.target as HTMLElement | null;
  const inField = !!target?.closest("input, textarea");
  const ctrl = e.ctrlKey && !e.altKey && !e.metaKey;
  const key = e.key.toLowerCase();
  const item = app.selectedItem;

  const run = (fn: () => unknown) => {
    e.preventDefault();
    void fn();
  };

  if (ctrl && !e.shiftKey && key === "f") return run(focusSearch);
  if ((ctrl && !e.shiftKey && key === "r") || e.key === "F5") return run(() => app.refresh());
  if (ctrl && !e.shiftKey && (key === "q" || key === "w")) return run(windowActions.close);
  if (ctrl && (e.key === "?" || (e.shiftKey && e.key === "/"))) return run(() => (dialogs.shortcuts = true));
  if (ctrl && e.shiftKey && key === "n" && app.collections.status === "ready") {
    return run(() => (dialogs.newKeyring = true));
  }
  if (ctrl && !e.shiftKey && key === "n" && canAddItem()) {
    return run(() => (dialogs.editor = { mode: "create", collectionId: app.selectedCollection!.id }));
  }
  if (ctrl && !e.shiftKey && key === "e" && item) return run(() => (dialogs.editor = { mode: "edit", item }));
  // Ctrl+C copies the secret only when there's no text selection to copy.
  if (ctrl && !e.shiftKey && key === "c" && item && !inField && !window.getSelection()?.toString()) {
    return run(() => actions.copySecret(item));
  }
  const back = (e.key === "Escape" && !inField) || (e.altKey && e.key === "ArrowLeft");
  if (back && app.back()) e.preventDefault();
}
