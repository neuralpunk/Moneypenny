// Typed bindings for the Rust `keyring` commands (src-tauri/src/commands.rs).
// Shapes mirror the serde output of src-tauri/src/keyring.rs.

import { invoke, isTauri } from "@tauri-apps/api/core";

/** D-Bus object path, e.g. "/org/freedesktop/secrets/collection/login". */
export type ObjectPath = string;

export interface CollectionInfo {
  id: ObjectPath;
  label: string;
  locked: boolean;
  /** Holds the `default` alias: where apps store new secrets. */
  isDefault: boolean;
  /** The in-memory session keyring, cleared at logout. */
  isSession: boolean;
  /** Visible even while locked. */
  itemCount: number;
  /** Seconds since the Unix epoch; 0 if unknown. */
  created: number;
  modified: number;
}

export interface ItemInfo {
  id: ObjectPath;
  collectionId: ObjectPath;
  label: string;
  locked: boolean;
  attributes: Record<string, string>;
  /** The `xdg:schema` attribute, if present. */
  schema: string | null;
  created: number;
  modified: number;
}

export type SecretValue =
  | { type: "text"; value: string }
  | { type: "binary"; size: number };

export type KeyringError =
  | { kind: "unavailable"; message: string }
  | { kind: "locked" }
  | { kind: "not-found" }
  | { kind: "dismissed" }
  | { kind: "invalid"; message: string }
  | { kind: "backend"; message: string };

const KINDS = new Set(["unavailable", "locked", "not-found", "dismissed", "invalid", "backend"]);

/** A short, human message for a toast. */
export function errorMessage(e: KeyringError): string {
  switch (e.kind) {
    case "unavailable":
      return "Can't reach the keyring service";
    case "locked":
      return "The keyring is locked";
    case "not-found":
      return "It no longer exists";
    case "dismissed":
      return "Cancelled";
    case "invalid":
    case "backend":
      return e.message;
  }
}

export function isKeyringError(e: unknown): e is KeyringError {
  return typeof e === "object" && e !== null && KINDS.has((e as { kind?: string }).kind ?? "");
}

/** Normalizes anything thrown by `invoke` into a KeyringError. */
export function toKeyringError(e: unknown): KeyringError {
  if (isKeyringError(e)) return e;
  return { kind: "backend", message: e instanceof Error ? e.message : String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    if (import.meta.env.DEV && !isTauri()) {
      const { mockInvoke } = await import("./mock");
      return (await mockInvoke(cmd, args)) as T;
    }
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toKeyringError(e);
  }
}

export const keyring = {
  listCollections: () => call<CollectionInfo[]>("list_collections"),
  listItems: (collectionId: ObjectPath) => call<ItemInfo[]>("list_items", { collectionId }),
  /** Fetch on demand only; don't keep the result around longer than needed. */
  getSecret: (collectionId: ObjectPath, itemId: ObjectPath) =>
    call<SecretValue>("get_secret", { collectionId, itemId }),

  // Unlock, create and change-password show the system password prompt;
  // they reject with { kind: "dismissed" } if the user cancels it.
  unlock: (collectionId: ObjectPath) => call<void>("unlock_collection", { collectionId }),
  lock: (collectionId: ObjectPath) => call<void>("lock_collection", { collectionId }),
  /** Returns the new keyring's id. */
  createCollection: (label: string) => call<ObjectPath>("create_collection", { label }),
  deleteCollection: (collectionId: ObjectPath) => call<void>("delete_collection", { collectionId }),
  changePassword: (collectionId: ObjectPath) => call<void>("change_password", { collectionId }),

  createItem: (collectionId: ObjectPath, label: string, attributes: Record<string, string>, secret: string) =>
    call<ItemInfo>("create_item", { collectionId, label, attributes, secret }),
  /** Omitted fields are left unchanged. */
  updateItem: (
    collectionId: ObjectPath,
    itemId: ObjectPath,
    changes: { label?: string; attributes?: Record<string, string>; secret?: string },
  ) => call<ItemInfo>("update_item", { collectionId, itemId, ...changes }),
  deleteItem: (collectionId: ObjectPath, itemId: ObjectPath) => call<void>("delete_item", { collectionId, itemId }),
};

export interface CopyResult {
  /** Seconds until the clipboard is cleared. */
  clearAfterSecs: number;
}

/** Copies a text secret to the clipboard in Rust; the value never reaches JS. */
export const copySecret = (collectionId: ObjectPath, itemId: ObjectPath) =>
  call<CopyResult>("copy_secret", { collectionId, itemId });
