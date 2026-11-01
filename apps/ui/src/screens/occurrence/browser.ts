// What the report keeps in the window itself: the ticked steps of each occurrence, and the clipboard.

const key = (id: number) => `vestigium.occurrence.${id}.done`;

/** The steps ticked on this occurrence; reading state, so it stays in the window, not in settings. */
export function loadDone(id: number): string[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key(id)) ?? "[]");
    return Array.isArray(value) ? value.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

export function saveDone(id: number, done: string[]): void {
  try {
    if (done.length) localStorage.setItem(key(id), JSON.stringify(done));
    else localStorage.removeItem(key(id));
  } catch {
    // Without storage the ticks last until the screen closes.
  }
}

/** Copies `text`, falling back to a hidden text area where the clipboard API is refused. */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const focused = document.activeElement as HTMLElement | null;
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.append(area);
    area.select();
    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch {
      copied = false;
    }
    area.remove();
    focused?.focus({ preventScroll: true });
    return copied;
  }
}
