const SHOWN_MS = 2600;

class Toast {
  message = $state("");
  on = $state(false);
  #timer: ReturnType<typeof setTimeout> | undefined;

  show(message: string): void {
    this.message = message;
    this.on = true;
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => (this.on = false), SHOWN_MS);
  }
}

export const toast = new Toast();
