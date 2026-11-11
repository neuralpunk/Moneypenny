# Moneypenny — Plan & Tracker

Moneypenny is a Linux-only GNOME-style GUI for managing secrets stored by
**oo7** (`oo7-daemon`, the Secret Service provider that replaced
gnome-keyring in Fedora 45).

## Key decisions

| Decision | Choice | Why |
|---|---|---|
| Backend | The `oo7` Rust crate (`oo7::dbus`), talking to `oo7-daemon` over the Secret Service D-Bus API | oo7 isn't CLI-only. The crate is the same project's own client library. It gives typed data and change signals, and unlock prompts go through the daemon's system prompter. Running `oo7-cli` and parsing its text would be fragile. |
| Shell | Tauri 2 (WebKitGTK 4.1) | Uses the system's WebKitGTK, so the package stays small, and the backend is Rust, like oo7. |
| Frontend | Svelte 5 (runes) + Vite + TypeScript, Tailwind 4 | Plain Vite SPA, no SvelteKit: it's a single window with no routing or SSR. |
| GNOME look | Undecorated window with a hand-built libadwaita-style header bar; Adwaita colour tokens; Adwaita Sans/Mono; bundled Adwaita symbolic icons | WebKitGTK can't use libadwaita widgets, so we copy the libadwaita 1.6+ design closely. |
| Theme sync | Read `color-scheme`, `accent-color` and `button-layout` from the XDG Settings portal (`ashpd`) and follow changes live | Matches the user's dark mode, accent colour and window-button layout, like a real libadwaita app does. |
| Secret safety | Secrets fetched only on demand, never cached in the frontend; clipboard auto-cleared; no secrets in logs | It's a password manager. |

## Phases

Status: ⬜ not started · 🟨 in progress · ✅ done

### ✅ Phase 1 — Scaffold & native-looking shell
- git repo, Tauri 2 + Svelte 5 + Tailwind 4 project, lint/type-check scripts
- Adwaita design tokens (light/dark, accent) in CSS
- Undecorated, rounded window; libadwaita-style split view (sidebar + content)
  with header bars, drag regions and window controls following `button-layout`
- Rust `desktop` module: portal settings → frontend, live updates via events
- App icon, README

### ✅ Phase 2 — Secret Service backend (read-only)
- `keyring` module on the `oo7::dbus` client: list collections (label, alias/default,
  locked, item count), list items (label, attributes, created/modified, locked),
  fetch one secret on demand
- Serializable DTOs and a typed error enum for Tauri commands
- Tests against a throwaway `oo7-daemon` on a private D-Bus session (never the
  user's real keyring)

Notes from this phase:
- Tauri commands: `list_collections`, `list_items(collectionId)`,
  `get_secret(collectionId, itemId)`. TypeScript bindings are in `src/lib/keyring.ts`.
  Errors arrive as `{ kind: "unavailable" | "locked" | "not-found" | "dismissed" | "backend" }`.
- **oo7-daemon hides item labels, attributes and timestamps while a keyring is
  locked**; only the keyring's label and item count are visible. `list_items`
  returns `locked` instead of partial data, so the UI shows an "unlock" page.
- Secrets that are valid, printable UTF-8 are shown as text even if stored as
  blobs; real binary secrets only report their size. Secret text is zeroized on
  drop on the Rust side and redacted from `Debug` output.
- The connection is created lazily and rebuilt once automatically if the daemon
  goes away (e.g. restarts).
- Test isolation: each test gets its own `dbus-daemon` (no service activation) and
  `oo7-daemon --login` with `HOME`, `XDG_*` and `OO7_PAM_SOCKET` all pointing into
  a temp dir under `$XDG_RUNTIME_DIR`. Override the daemon with `OO7_DAEMON=…`.
  Tests skip if it's not installed.

### ✅ Phase 3 — Browsing UI
- Sidebar of keyrings (lock state, default badge), item list with search
- Item detail page (boxed-list rows: label, attributes, dates)
- Reveal/hide and copy secret; clipboard auto-clears after 15 s (was 30 s)
- Empty, locked and error status pages

Notes from this phase:
- Layout: keyrings sidebar → item list (search in the header bar) → item page
  pushed over the list with a back button (AdwNavigationView-style). Esc goes back.
- Copying happens in Rust through GTK's clipboard (`clipboard.rs`), so the
  secret never reaches the page. After 15 s the clipboard is cleared, but
  only if it still holds that secret and no newer copy has replaced it.
- Revealed secrets live only in the secret row's state and are dropped on hide,
  item change or leaving the page.
- Widgets: Button, SearchEntry, ActionRow (incl. "property" style),
  PreferencesGroup / boxed list, Spinner, ToastOverlay.
- `pnpm dev` in a plain browser uses sample data (`src/lib/mock.ts`), which is
  never in production builds. `pnpm test` runs Vitest for the display helpers.
- `cargo run --example smoke` checks the real keyring read-only and prints only
  keyring names, lock state and counts. Rust logging uses `env_logger`
  (`RUST_LOG=…`).
- Locked keyrings show an explanatory page; the Unlock button comes in Phase 4.

### ✅ Phase 4 — Editing
- Unlock/lock keyrings (daemon's prompt)
- Create/edit/delete items; create/delete keyrings; ~~set default keyring~~ (blocked, see below)
- Change keyring password (oo7's gnome-keyring-compatible interface)
- libadwaita-style dialogs, confirmations for destructive actions, toasts

Notes from this phase:
- **Moneypenny never handles keyring passwords.** Unlock, create keyring and
  change password all go through oo7-daemon's prompt, which on GNOME is the
  shell's system dialog (`org.gnome.keyring.SystemPrompter`). Change password
  uses `ChangeWithPrompt` and unlocks first if needed.
- **Set default keyring is blocked on oo7-daemon 0.7.1**, for two reasons:
  `SetAlias("default", x)` doesn't remove the alias from the old default, so two
  keyrings claim it and `ReadAlias` picks one at random; and aliases aren't
  saved, since at startup the alias comes from the file name (`login.keyring` →
  default). The option would quietly revert at next login, so it isn't offered.
  Worth reporting upstream; revisit when fixed.
- New items never replace existing ones (`replace = false`) and get the
  `org.gnome.keyring.Note` schema if none is given (what Seahorse uses for
  passwords; shown as "Password or note").
- The editor hides internal `xdg:` attributes but keeps them on save.
- Mutations are never retried after a dropped connection (a retry could
  duplicate a change); reads still retry once.
- The session keyring has no keyring menu: no password to change, and locking
  or deleting it isn't useful.
- oo7 quirks found while testing: an **empty** keyring accepts any password
  (there's no encrypted item to check against); the CLI prompter never
  completes a prompt after a wrong password (the GNOME prompter retries up to 5
  times); cancelling the CLI prompter fails the call ("Prompt dismissed by
  user.") instead of completing it, so that's mapped to `dismissed`.
- Tests: `tests/editing.rs` runs prompt flows end-to-end. oo7-daemon picks the
  CLI prompter for non-graphical callers, so the harness gives the test daemon
  no system bus (no logind), re-runs prompt tests in a child process without
  `WAYLAND_DISPLAY`/`DISPLAY`, and answers prompts with a fake
  `org.freedesktop.secrets.CliPrompter`.

### ✅ Phase 5 — Live updates & polish
- Subscribe to Secret Service signals (collection/item created/changed/deleted)
  so changes made by other apps show up immediately
- Keyboard shortcuts (Ctrl+F, Ctrl+N, Ctrl+C, Esc, Ctrl+Q), focus rings, a11y labels
- Adaptive layout (collapse to single pane on narrow widths), About dialog

Notes from this phase:
- `watch.rs` listens to every signal under `/org/freedesktop/secrets` (except
  prompt `Completed`) plus `NameOwnerChanged` for the daemon. Signals within
  150 ms are merged and sent as a single `keyring-changed` event. The UI
  refreshes in place, and refreshes requested while one is running are merged
  into one more. Locking and unlocking are reported too.
- Shortcuts live in `src/lib/shortcuts.ts` (one table drives both the handler and
  the Keyboard Shortcuts dialog). Ctrl+C copies the secret only when no text
  is selected; shortcuts are ignored while a dialog or menu is open.
- Narrow layout below 600 px: one pane at a time with back buttons (Esc /
  Alt+← step back). Header bars use GTK center-box sizing so buttons are never
  covered by the title or search field.
- Keyboard: Up/Down move through the keyrings list and menus (Home/End too).
- Main menu (sidebar ☰): Keyboard Shortcuts, About Moneypenny.

### ✅ Phase 6 — Packaging
- `.desktop` file, AppStream metainfo, icons, RPM via the Tauri bundler
- Release build verification on Fedora 45

Notes from this phase:
- `pnpm tauri build` produces
  `src-tauri/target/release/bundle/rpm/Moneypenny-0.1.0-1.x86_64.rpm` (~3.4 MB).
  RPM is the only target (no AppImage, which would mean downloading extra
  build tools).
- Package contents: `/usr/bin/moneypenny`, `Moneypenny.desktop` (from
  `packaging/moneypenny.desktop.hbs`), hicolor icons 32/128/256/512 px plus a
  scalable SVG, AppStream metainfo, and the GPL-3.0 `LICENSE`.
- Dependencies: `webkit2gtk4.1` and `gtk3` (plus libraries rpm detects automatically);
  it **recommends** `oo7-daemon`, since any Secret Service works.
- App ID: the program name is set to `io.github.neuralpunk.Moneypenny` before GTK
  starts, so the window's app ID / WM_CLASS matches `StartupWMClass` in the
  `.desktop` file (checked with `xprop`: WM_CLASS is the app ID). Tauri's
  `enableGTKAppId` was tried and rejected: with it, a second launch re-runs
  setup inside the first instance and **crashes it** ("a webview with label
  `main` already exists"). Now each launch is an independent window.
- Release profile: LTO, one codegen unit, stripped.
- Validation: `desktop-file-validate` and `appstreamcli validate` pass (the
  metainfo links the GitHub page as homepage, bug tracker and source).

## Release 0.1.0 (2026-11-11)
- Published at https://github.com/neuralpunk/Moneypenny with an x86_64 RPM
  built on Fedora 45 (for Fedora 45 and later). RPM only: no deb or AppImage.

## Theme: Black Tie
Chosen from five James Bond–inspired studies (intensity 1 of 5): Adwaita
layout and widgets unchanged; deeper, slightly cooler neutrals; champagne gold
(`#cdb06a` dark / `#c9a961` light) as a fixed accent for buttons, focus and the
selected keyring; a 1 px gold trim under header bars. Tokens live in
`src/app.css`. The system accent colour is intentionally not applied any more
(the portal value is still read, so it could return as an option).

## Bug sweep (after Phase 6)
- **Clipboard outlived the app:** quitting before the clear timeout skipped the
  clear, and GNOME's compositor keeps clipboard contents after the owner
  exits. The secret is now also wiped on exit (`clipboard::clear_now`).
- **Prompts could hang forever** if oo7-daemon died mid-prompt (spinner stuck
  until restart). Prompt operations now fail with `unavailable` when the
  daemon's bus name changes (`Keyring::while_daemon_runs`). Regression test
  confirmed to fail without the fix.
- **First save after a daemon restart failed:** the cached encryption session
  belonged to the old daemon. The watcher now drops the cached connection when
  the daemon's bus name changes.
- **Listings failed if something was deleted mid-read;** vanished entries
  are now skipped.
- **Attribute names differing only by spaces** silently overwrote each other;
  now rejected as invalid.

## Follow-ups (not scheduled)
- Set default keyring once oo7-daemon saves aliases and clears the old default
  (see Phase 4 notes); worth reporting upstream.
- A real app icon (the current one is a placeholder).
- Screenshots in the AppStream metainfo (the README has one: `docs/screenshot.png`, taken from `pnpm dev` sample data).
