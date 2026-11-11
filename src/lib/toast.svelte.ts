// AdwToastOverlay equivalent: one toast at a time, newest replaces oldest.

export interface Toast {
  id: number;
  title: string;
}

const TIMEOUT_MS = 5000; // libadwaita's default toast timeout

class Toasts {
  current = $state<Toast | null>(null);
  #nextId = 1;
  #timer: ReturnType<typeof setTimeout> | undefined;

  show(title: string) {
    clearTimeout(this.#timer);
    const toast = { id: this.#nextId++, title };
    this.current = toast;
    this.#timer = setTimeout(() => this.dismiss(toast.id), TIMEOUT_MS);
  }

  dismiss(id?: number) {
    if (id === undefined || this.current?.id === id) {
      clearTimeout(this.#timer);
      this.current = null;
    }
  }
}

export const toasts = new Toasts();
