// Sample data for working on the UI in a plain browser (`pnpm dev`).
// Only loaded when not running inside Tauri in a dev build; never shipped.

import type { CollectionInfo, CopyResult, ItemInfo, KeyringError, SecretValue } from "./keyring";

const now = Math.floor(Date.now() / 1000);
const C = "/org/freedesktop/secrets/collection";

const collections: CollectionInfo[] = [
  { id: `${C}/login`, label: "Login", locked: false, isDefault: true, isSession: false, itemCount: 6, created: now - 9e6, modified: now - 3600 },
  { id: `${C}/work`, label: "Work", locked: true, isDefault: false, isSession: false, itemCount: 3, created: now - 5e6, modified: now - 86400 },
  { id: `${C}/session`, label: "Session", locked: false, isDefault: false, isSession: true, itemCount: 0, created: now - 600, modified: now - 600 },
];

function item(n: number, label: string, schema: string | null, attributes: Record<string, string>): ItemInfo {
  return {
    id: `${C}/login/${n}`,
    collectionId: `${C}/login`,
    label,
    locked: false,
    schema,
    attributes: schema ? { "xdg:schema": schema, ...attributes } : attributes,
    created: now - n * 86400 * 30,
    modified: now - n * 86400,
  };
}

const items: ItemInfo[] = [
  item(1, "Chrome Safe Storage", "chrome_libsecret_os_crypt_password_v2", { application: "chrome" }),
  item(2, "GitHub", "org.gnome.keyring.NetworkPassword", { user: "alice", server: "github.com", protocol: "https" }),
  item(3, "Network secret for Home Wi-Fi/802-11-wireless-security/psk", "org.freedesktop.NetworkManager.Connection", { "connection-uuid": "8c3e…", "setting-name": "802-11-wireless-security", "setting-key": "psk" }),
  item(4, "GOA google credentials for identity account_1", "org.gnome.OnlineAccounts", { "goa-identity": "google:gen0:account_1" }),
  item(5, "Fractal: matrix.org access token", "org.gnome.Fractal", { username: "@alice:matrix.org", "xdg:content-type": "text/plain" }),
  item(6, "SSH key passphrase", null, { unique: "ssh-store:/home/alice/.ssh/id_ed25519" }),
];

const delay = <T>(v: T) => new Promise<T>((r) => setTimeout(() => r(v), 150));
const fail = (e: KeyringError) => Promise.reject(e);
let nextItem = 100;

function counts() {
  for (const c of collections) {
    if (!c.locked) c.itemCount = items.filter((i) => i.collectionId === c.id).length;
  }
}

export function mockInvoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  const c = collections.find((x) => x.id === args.collectionId);
  const item = items.find((i) => i.id === args.itemId);
  switch (cmd) {
    case "list_collections":
      counts();
      return delay(structuredClone(collections));
    case "list_items":
      if (!c) return fail({ kind: "not-found" });
      if (c.locked) return fail({ kind: "locked" });
      return delay(structuredClone(items.filter((i) => i.collectionId === c.id)));
    case "get_secret":
      return delay<SecretValue>({ type: "text", value: "correct horse battery staple" });
    case "copy_secret":
      return delay<CopyResult>({ clearAfterSecs: 15 });
    case "unlock_collection":
    case "lock_collection":
      if (!c) return fail({ kind: "not-found" });
      c.locked = cmd === "lock_collection";
      return delay(undefined);
    case "change_password":
      return delay(undefined);
    case "create_collection": {
      const label = String(args.label).trim();
      const id = `${C}/${label.toLowerCase().replace(/\W/g, "_")}`;
      collections.splice(collections.length - 1, 0, { id, label, locked: false, isDefault: false, isSession: false, itemCount: 0, created: now, modified: now });
      return delay(id);
    }
    case "delete_collection":
      collections.splice(collections.findIndex((x) => x.id === args.collectionId), 1);
      return delay(undefined);
    case "create_item": {
      if (!c) return fail({ kind: "not-found" });
      const attributes = { "xdg:schema": "org.gnome.keyring.Note", ...(args.attributes as Record<string, string>) };
      const created: ItemInfo = { id: `${c.id}/${nextItem++}`, collectionId: c.id, label: String(args.label).trim(), locked: false, schema: attributes["xdg:schema"] ?? null, attributes, created: now, modified: now };
      items.push(created);
      return delay(created);
    }
    case "update_item": {
      if (!item) return fail({ kind: "not-found" });
      if (typeof args.label === "string") item.label = args.label.trim();
      if (args.attributes) {
        item.attributes = args.attributes as Record<string, string>;
        item.schema = item.attributes["xdg:schema"] ?? null;
      }
      item.modified = Math.floor(Date.now() / 1000);
      return delay(structuredClone(item));
    }
    case "delete_item":
      items.splice(items.findIndex((i) => i.id === args.itemId), 1);
      return delay(undefined);
    default:
      return fail({ kind: "backend", message: `mock: unknown command ${cmd}` });
  }
}
