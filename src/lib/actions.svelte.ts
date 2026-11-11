// User actions that change keyrings: each shows feedback, then refreshes.

import { SvelteSet } from "svelte/reactivity";
import { collectionLabel } from "./format";
import {
  copySecret,
  errorMessage,
  keyring,
  toKeyringError,
  type CollectionInfo,
  type ItemInfo,
  type KeyringError,
} from "./keyring";
import { app } from "./state.svelte";
import { toasts } from "./toast.svelte";

export type Outcome<T> = { ok: true; value: T } | { ok: false; error: KeyringError };

/** Keys of actions in progress, e.g. "unlock:/org/…", for spinners. */
export const pending = new SvelteSet<string>();

interface Options<T> {
  success?: string | ((value: T) => string);
  /** Don't toast errors (the caller shows them, e.g. inside a dialog). */
  quiet?: boolean;
}

async function run<T>(key: string, op: () => Promise<T>, opts: Options<T> = {}): Promise<Outcome<T>> {
  pending.add(key);
  try {
    const value = await op();
    if (opts.success) toasts.show(typeof opts.success === "function" ? opts.success(value) : opts.success);
    return { ok: true, value };
  } catch (e) {
    const error = toKeyringError(e);
    // Cancelling a password prompt is a choice, not an error.
    if (error.kind !== "dismissed" && !opts.quiet) toasts.show(errorMessage(error));
    return { ok: false, error };
  } finally {
    pending.delete(key);
    await app.refresh();
  }
}

export const actions = {
  /** Copies in Rust; the secret never reaches the page. No refresh needed. */
  async copySecret(item: ItemInfo) {
    try {
      const { clearAfterSecs } = await copySecret(item.collectionId, item.id);
      toasts.show(`Copied. The clipboard will be cleared in ${clearAfterSecs} seconds.`);
    } catch (e) {
      const error = toKeyringError(e);
      if (error.kind === "locked" || error.kind === "not-found") void app.refresh();
      toasts.show(errorMessage(error));
    }
  },

  unlock: (c: CollectionInfo) => run(`unlock:${c.id}`, () => keyring.unlock(c.id)),

  lock: (c: CollectionInfo) =>
    run(`lock:${c.id}`, () => keyring.lock(c.id), { success: `“${collectionLabel(c)}” locked` }),

  changePassword: (c: CollectionInfo) =>
    run(`password:${c.id}`, () => keyring.changePassword(c.id), { success: "Password changed" }),

  async createKeyring(label: string) {
    const result = await run("create-keyring", () => keyring.createCollection(label), {
      success: `“${label.trim()}” created`,
    });
    if (result.ok) await app.selectCollection(result.value);
    return result;
  },

  deleteKeyring: (c: CollectionInfo) =>
    run(`delete:${c.id}`, () => keyring.deleteCollection(c.id), {
      success: `“${collectionLabel(c)}” deleted`,
    }),

  async createItem(collectionId: string, label: string, attributes: Record<string, string>, secret: string) {
    const result = await run("save-item", () => keyring.createItem(collectionId, label, attributes, secret), {
      success: "Secret added",
      quiet: true,
    });
    if (result.ok) app.openItem(result.value.id);
    return result;
  },

  updateItem: (
    item: ItemInfo,
    changes: { label?: string; attributes?: Record<string, string>; secret?: string },
  ) =>
    run("save-item", () => keyring.updateItem(item.collectionId, item.id, changes), {
      success: "Changes saved",
      quiet: true,
    }),

  async deleteItem(item: ItemInfo) {
    app.closeItem();
    return run(`delete:${item.id}`, () => keyring.deleteItem(item.collectionId, item.id), {
      success: `“${item.label || "Untitled"}” deleted`,
    });
  },
};
