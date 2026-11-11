// Which dialog is open. Rendered by views/Dialogs.svelte.

import type { ItemInfo } from "./keyring";

export type EditorRequest = { mode: "create"; collectionId: string } | { mode: "edit"; item: ItemInfo };

export interface ConfirmRequest {
  heading: string;
  body: string;
  confirm: string;
  destructive?: boolean;
  resolve: (ok: boolean) => void;
}

class Dialogs {
  newKeyring = $state(false);
  about = $state(false);
  shortcuts = $state(false);
  editor = $state<EditorRequest | null>(null);
  confirm = $state<ConfirmRequest | null>(null);

  /** AdwAlertDialog-style confirmation; resolves true if confirmed. */
  ask(req: Omit<ConfirmRequest, "resolve">): Promise<boolean> {
    return new Promise((resolve) => {
      this.confirm?.resolve(false);
      this.confirm = { ...req, resolve };
    });
  }
}

export const dialogs = new Dialogs();
