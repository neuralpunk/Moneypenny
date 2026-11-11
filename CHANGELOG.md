# Changelog

## 0.1.0 (2026-11-11)

First release, for Fedora 45 and later.

- Browse keyrings and the secrets in them; search by name or attribute.
- Reveal or copy a secret. Copying goes through GTK's clipboard from Rust, and
  the clipboard is cleared after 15 seconds, or when Moneypenny quits, if it
  still holds the secret.
- Add, edit and delete secrets; create, lock, unlock and delete keyrings;
  change a keyring's password. Passwords are always entered in the system's
  own dialog, never in Moneypenny.
- Updates live when other apps change your keyrings, and recovers when
  oo7-daemon restarts.
- "Black Tie" theme on an Adwaita layout; follows GNOME's dark/light style,
  high contrast, reduced motion and window-button layout; adapts to narrow
  windows.
- Keyboard shortcuts (<kbd>Ctrl</kbd>+<kbd>?</kbd> lists them).
- RPM package with a desktop entry, icons and AppStream metainfo.

Known limitation: setting the default keyring isn't offered yet, because
oo7-daemon 0.7 doesn't save that choice (see docs/PLAN.md).
