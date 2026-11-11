import { afterEach, beforeEach } from "vitest";

import { setLang } from "../lib/i18n.svelte";
import { resetIpc } from "./test-ipc";
import { unmountAll } from "./test-mount";

// jsdom has no media queries or resize observers; the window reads both when it starts.
window.matchMedia ??= (query: string) =>
  ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
    onchange: null,
  }) as MediaQueryList;
globalThis.ResizeObserver ??= class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
};

// jsdom does not implement the modal methods of <dialog>; they set and clear the open attribute.
const dialog = HTMLDialogElement.prototype;
dialog.showModal ??= function (this: HTMLDialogElement) {
  this.setAttribute("open", "");
};
dialog.close ??= function (this: HTMLDialogElement) {
  this.removeAttribute("open");
};

beforeEach(() => {
  setLang("en");
});

afterEach(() => {
  unmountAll();
  resetIpc();
});
