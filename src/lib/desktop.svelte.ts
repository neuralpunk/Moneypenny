// Desktop integration state: theme, accent, contrast, motion and window-button
// layout from the GNOME Settings portal (via the Rust `desktop` module), plus
// window maximized/focused state.

import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type ColorScheme = "default" | "prefer-dark" | "prefer-light";

export interface DesktopSettings {
  colorScheme: ColorScheme;
  accentColor: [number, number, number] | null;
  highContrast: boolean;
  reducedMotion: boolean;
  buttonLayout: string;
}

export type WindowButton = "minimize" | "maximize" | "close";

export interface ButtonLayout {
  start: WindowButton[];
  end: WindowButton[];
}

const KNOWN_BUTTONS = new Set<string>(["minimize", "maximize", "close"]);

/** Parses GNOME's `button-layout`, e.g. "appmenu:minimize,maximize,close". */
export function parseButtonLayout(layout: string): ButtonLayout {
  const [start = "", end = ""] = layout.split(":");
  const pick = (side: string) =>
    side
      .split(",")
      .map((b) => b.trim())
      .filter((b): b is WindowButton => KNOWN_BUTTONS.has(b));
  return { start: pick(start), end: pick(end) };
}

const FALLBACK: DesktopSettings = {
  colorScheme: "default",
  accentColor: null,
  highContrast: false,
  reducedMotion: false,
  buttonLayout: "appmenu:close",
};

class Desktop {
  settings = $state<DesktopSettings>(FALLBACK);
  maximized = $state(false);
  focused = $state(true);
  /** Below this width the split view collapses to one pane at a time. */
  narrow = $state(false);

  dark = $derived(this.settings.colorScheme === "prefer-dark");
  buttons = $derived(parseButtonLayout(this.settings.buttonLayout));
}

export const desktop = new Desktop();

function applyToDocument(s: DesktopSettings) {
  const root = document.documentElement;
  root.dataset.scheme = s.colorScheme === "prefer-dark" ? "dark" : "light";
  root.toggleAttribute("data-reduced-motion", s.reducedMotion);
  if (s.highContrast) root.dataset.contrast = "high";
  else delete root.dataset.contrast;

  // The system accent colour (s.accentColor) is deliberately not applied:
  // the Black Tie palette fixes the accent to champagne gold (see app.css).
}

function update(s: DesktopSettings) {
  desktop.settings = s;
  applyToDocument(s);
}

const NARROW_QUERY = "(max-width: 599px)";

export async function initDesktop() {
  const narrow = window.matchMedia(NARROW_QUERY);
  desktop.narrow = narrow.matches;
  narrow.addEventListener("change", () => (desktop.narrow = narrow.matches));

  // Outside Tauri (e.g. `pnpm dev` in a browser) fall back to media queries.
  if (!isTauri()) {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const sync = () =>
      update({ ...FALLBACK, buttonLayout: "appmenu:minimize,maximize,close", colorScheme: mq.matches ? "prefer-dark" : "prefer-light" });
    mq.addEventListener("change", sync);
    sync();
    return;
  }

  applyToDocument(FALLBACK);
  await listen<DesktopSettings>("desktop-settings-changed", (e) => update(e.payload));
  update(await invoke<DesktopSettings>("desktop_settings"));

  // Real apps don't show WebKit's "Reload / Inspect" menu.
  if (!import.meta.env.DEV) {
    document.addEventListener("contextmenu", (e) => {
      const t = e.target as HTMLElement | null;
      if (!t?.closest("input, textarea")) e.preventDefault();
    });
  }

  const win = getCurrentWindow();
  const syncMaximized = async () => {
    desktop.maximized = await win.isMaximized();
    document.documentElement.toggleAttribute("data-maximized", desktop.maximized);
  };
  await win.onResized(syncMaximized);
  await win.onFocusChanged(({ payload }) => {
    desktop.focused = payload;
  });
  await syncMaximized();
}

export const windowActions = {
  minimize: () => getCurrentWindow().minimize(),
  maximize: () => getCurrentWindow().toggleMaximize(),
  close: () => getCurrentWindow().close(),
};
