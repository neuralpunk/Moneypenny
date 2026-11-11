// App state: keyrings, the selected keyring's items, and navigation.

import { keyring, toKeyringError, type CollectionInfo, type ItemInfo, type KeyringError } from "./keyring";
import { matchesQuery } from "./format";

export type Load<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: KeyringError };

class AppState {
  collections = $state<Load<CollectionInfo[]>>({ status: "loading" });
  items = $state<Load<ItemInfo[]>>({ status: "loading" });
  selectedCollectionId = $state<string | null>(null);
  selectedItemId = $state<string | null>(null);
  query = $state("");
  refreshing = $state(false);
  /** Narrow (single-pane) layout: whether the content pane is showing. */
  showContent = $state(false);

  #inflight: Promise<void> | null = null;
  #again = false;

  /** Guards against out-of-order responses when switching quickly. */
  #itemsToken = 0;

  selectedCollection = $derived(
    this.collections.status === "ready"
      ? (this.collections.data.find((c) => c.id === this.selectedCollectionId) ?? null)
      : null,
  );

  selectedItem = $derived(
    this.items.status === "ready"
      ? (this.items.data.find((i) => i.id === this.selectedItemId) ?? null)
      : null,
  );

  filteredItems = $derived(
    this.items.status === "ready" ? this.items.data.filter((i) => matchesQuery(i, this.query)) : [],
  );

  /**
   * Loads (or reloads) keyrings, keeping the current selection if it still
   * exists. Calls made while a refresh is running are merged into one more.
   */
  refresh(): Promise<void> {
    if (this.#inflight) {
      this.#again = true;
      return this.#inflight;
    }
    this.#inflight = (async () => {
      do {
        this.#again = false;
        await this.#refreshOnce();
      } while (this.#again);
    })().finally(() => {
      this.#inflight = null;
    });
    return this.#inflight;
  }

  async #refreshOnce() {
    this.refreshing = true;
    try {
      const data = await keyring.listCollections();
      this.collections = { status: "ready", data };
      const stillThere = data.some((c) => c.id === this.selectedCollectionId);
      if (!stillThere) {
        this.selectedCollectionId = (data.find((c) => c.isDefault) ?? data[0])?.id ?? null;
        this.selectedItemId = null;
        this.query = "";
      }
      await this.loadItems({ keepVisible: stillThere });
    } catch (e) {
      this.collections = { status: "error", error: toKeyringError(e) };
    } finally {
      this.refreshing = false;
    }
  }

  async selectCollection(id: string) {
    this.showContent = true;
    if (id === this.selectedCollectionId && this.selectedItemId === null) return;
    this.selectedCollectionId = id;
    this.selectedItemId = null;
    this.query = "";
    await this.loadItems();
  }

  /** `keepVisible` avoids flashing a spinner when refreshing the same keyring. */
  async loadItems({ keepVisible = false } = {}) {
    const id = this.selectedCollectionId;
    const token = ++this.#itemsToken;
    if (!id) {
      this.items = { status: "ready", data: [] };
      return;
    }
    if (!keepVisible || this.items.status !== "ready") this.items = { status: "loading" };
    try {
      const data = await keyring.listItems(id);
      if (token !== this.#itemsToken) return;
      this.items = { status: "ready", data };
      if (this.selectedItemId && !data.some((i) => i.id === this.selectedItemId)) {
        this.selectedItemId = null;
      }
    } catch (e) {
      if (token !== this.#itemsToken) return;
      this.items = { status: "error", error: toKeyringError(e) };
      this.selectedItemId = null;
    }
  }

  openItem(id: string) {
    this.selectedItemId = id;
  }

  closeItem() {
    this.selectedItemId = null;
  }

  /** One step back: item → list → (narrow layout) keyrings. */
  back(): boolean {
    if (this.selectedItemId) {
      this.closeItem();
      return true;
    }
    if (this.showContent) {
      this.showContent = false;
      return true;
    }
    return false;
  }
}

export const app = new AppState();
