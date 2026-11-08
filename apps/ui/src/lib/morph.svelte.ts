import { tick } from "svelte";

import { buildGroups, groupOf } from "./groups";
import { router, type Route } from "./router.svelte";
import { toast } from "./toast.svelte";

// A home card grows into the page and the page shrinks back into its card; only transform and opacity move.

export type Phase = "idle" | "opening" | "closing";

const OPEN_MS = 440;
const CLOSE_MS = 360;
const EASE = "cubic-bezier(.5, 0, .15, 1)";
const EASE_CLOSE = "cubic-bezier(.85, 0, .5, 1)";

const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

let pixel: CanvasRenderingContext2D | null = null;

/** Paints the colours over each other on one pixel and returns its RGBA. */
function paint(colors: string[]): Uint8ClampedArray {
  pixel ??= Object.assign(document.createElement("canvas"), { width: 1, height: 1 }).getContext("2d", { willReadFrequently: true })!;
  pixel.clearRect(0, 0, 1, 1);
  for (const color of colors) {
    pixel.fillStyle = color;
    pixel.fillRect(0, 0, 1, 1);
  }
  return pixel.getImageData(0, 0, 1, 1).data;
}

/** The opaque colour an element shows: its background over its ancestors', down to the first opaque one. */
function solidOf(el: Element): string {
  const layers: string[] = [];
  for (let node: Element | null = el; node; node = node.parentElement) {
    const color = getComputedStyle(node).backgroundColor;
    layers.unshift(color);
    if (paint([color])[3] === 255) break;
  }
  const [r, g, b] = paint(layers);
  return `rgb(${r}, ${g}, ${b})`;
}

const reverse = (frames: Keyframe[]): Keyframe[] =>
  frames
    .slice()
    .reverse()
    .map((f) => (f.offset == null ? f : { ...f, offset: 1 - f.offset }));

class Morph {
  phase = $state<Phase>("idle");
  stage: HTMLElement | null = null;
  home: HTMLElement | null = null;
  page: HTMLElement | null = null;

  #origin: { id: string; group: string | null } | null = null;
  #animations: Animation[] = [];
  #copy: HTMLElement | null = null;
  #settle: (() => void) | null = null;

  /** Notes which card was clicked, so the way back lands on the same one. */
  remember(id: string): void {
    this.#origin = { id, group: null };
  }

  /** Router hook: plays the transition between the home screen and a page. */
  readonly hook = (next: Route, _prev: Route, commit: () => void): boolean => {
    this.#finish();
    const from = router.current;
    const opening = from.screen === "home" && next.screen !== "home";
    const closing = from.screen !== "home" && next.screen === "home";
    if ((!opening && !closing) || reduced() || !this.stage) {
      this.#origin = null;
      return false;
    }
    void (opening ? this.#open(next, commit) : this.#close(from, commit));
    return true;
  };

  /** Ends a running transition at once, committing the route it was heading to. */
  #finish(): void {
    if (this.phase === "idle") return;
    this.#clear();
    const settle = this.#settle;
    this.#settle = null;
    settle?.();
    this.phase = "idle";
  }

  #clear(): void {
    this.#animations.forEach((a) => a.cancel());
    this.#animations = [];
    this.#copy?.remove();
    this.#copy = null;
    if (this.page) {
      this.page.style.transformOrigin = "";
      this.page.style.willChange = "";
    }
  }

  async #open(next: Route, commit: () => void): Promise<void> {
    const groups = buildGroups();
    const group = groupOf(groups, next, null)?.key ?? null;
    if (this.#origin) this.#origin.group = group;
    const card = this.#cardFor(this.#origin?.id ?? null, group);
    const rect = card?.getBoundingClientRect() ?? null;
    this.phase = "opening";
    this.#settle = commit;
    commit();
    await tick();
    if (this.phase !== "opening" || !this.page || !this.home) return;
    await this.#run(this.page, card, rect, OPEN_MS, false);
    this.#end();
  }

  async #close(prev: Route, commit: () => void): Promise<void> {
    this.phase = "closing";
    this.#settle = commit;
    await tick();
    if (this.phase !== "closing" || !this.page || !this.home) return;
    const group = groupOf(buildGroups(), prev, router.group)?.key ?? null;
    const origin = this.#origin && this.#origin.group === group ? this.#origin.id : null;
    const card = this.#cardFor(origin, group);
    await this.#run(this.page, card, card?.getBoundingClientRect() ?? null, CLOSE_MS, true);
    this.#end();
  }

  #end(): void {
    if (this.phase === "idle") return;
    this.#clear();
    const settle = this.#settle;
    this.#settle = null;
    settle?.();
    this.#origin = null;
    this.phase = "idle";
  }

  #cardFor(id: string | null, group: string | null): HTMLElement | null {
    const find = (key: string) => this.home?.querySelector<HTMLElement>(`[data-morph="${key}"]`) ?? null;
    return (id && find(id)) || (group === "settings" ? find("qs:settings") : null) || (group ? find(`g:${group}`) : null);
  }

  #play(el: Element, frames: Keyframe[], duration: number, easing = "linear"): Promise<Animation> {
    const animation = el.animate(frames, { duration, easing, fill: "both" });
    this.#animations.push(animation);
    return animation.finished;
  }

  async #run(page: HTMLElement, card: HTMLElement | null, rect: DOMRect | null, duration: number, closing: boolean) {
    const probe = this.#probe(closing ? "close" : "open");
    if (!card || !rect || !this.stage) {
      await this.#within(duration, [
        this.#play(page, closing ? [{ opacity: 1 }, { opacity: 0 }] : [{ opacity: 0 }, { opacity: 1 }], duration, EASE),
      ]);
      probe();
      return;
    }
    const stage = this.stage.getBoundingClientRect();
    const x = rect.left - stage.left;
    const y = rect.top - stage.top;
    const sx = rect.width / stage.width;
    const sy = rect.height / stage.height;
    const pageFrom = `translate(${x}px, ${y}px) scale(${sx}, ${sy})`;
    const copyTo = `translate(${-x}px, ${-y}px) scale(${1 / sx}, ${1 / sy})`;

    const background = solidOf(card);
    const copy = card.cloneNode(true) as HTMLElement;
    ["href", "data-morph"].forEach((a) => copy.removeAttribute(a));
    copy.setAttribute("aria-hidden", "true");
    copy.tabIndex = -1;
    copy.classList.add("morph");
    Object.assign(copy.style, {
      left: `${x}px`,
      top: `${y}px`,
      width: `${rect.width}px`,
      height: `${rect.height}px`,
      background,
      boxShadow: "none",
      borderRadius: "0",
      overflow: "visible",
    });
    this.stage.append(copy);
    this.#copy = copy;

    const dir = (frames: Keyframe[]) => (closing ? reverse(frames) : frames);
    const ease = closing ? EASE_CLOSE : EASE;
    page.style.transformOrigin = "0 0";
    page.style.willChange = "transform, opacity";
    await this.#within(duration, [
      this.#play(page, dir([{ transform: pageFrom, offset: 0 }, { transform: "none", offset: 1 }]), duration, ease),
      this.#play(
        page,
        dir([{ opacity: 0, offset: 0 }, { opacity: 0, offset: 0.1 }, { opacity: 1, offset: 0.4 }, { opacity: 1, offset: 1 }]),
        duration,
      ),
      this.#play(copy, dir([{ transform: "none", offset: 0 }, { transform: copyTo, offset: 1 }]), duration, ease),
      this.#play(
        copy,
        dir([{ opacity: 1, offset: 0 }, { opacity: 1, offset: 0.4 }, { opacity: 0, offset: 0.85 }, { opacity: 0, offset: 1 }]),
        duration,
      ),
      ...[...copy.children].map((child) =>
        this.#play(
          child,
          dir([{ opacity: 1, offset: 0 }, { opacity: 1, offset: 0.08 }, { opacity: 0, offset: 0.32 }, { opacity: 0, offset: 1 }]),
          duration,
        ),
      ),
    ]);
    probe();
  }

  /** Waits for the animations, but never longer than their length plus a margin: a hidden window may not tick. */
  async #within(duration: number, running: Promise<unknown>[]): Promise<void> {
    await Promise.race([
      Promise.all(running).catch(() => undefined),
      new Promise((resolve) => setTimeout(resolve, duration + 400)),
    ]);
  }

  /** With `?perf` in the address, reports the frame times of each transition. */
  #probe(label: string): () => void {
    if (!new URLSearchParams(location.search).has("perf")) return () => undefined;
    const times: number[] = [];
    let on = true;
    const frame = (now: number) => {
      times.push(now);
      if (on) requestAnimationFrame(frame);
    };
    requestAnimationFrame(frame);
    return () => {
      on = false;
      const deltas = times.slice(1).map((v, i) => v - times[i]);
      if (!deltas.length) return;
      const avg = deltas.reduce((a, b) => a + b, 0) / deltas.length;
      const message = `${label}: ${deltas.length} frames, mean ${avg.toFixed(1)} ms, worst ${Math.max(...deltas).toFixed(0)} ms`;
      console.log("[morph]", message);
      toast.show(message);
    };
  }
}

export const morph = new Morph();
router.hook = morph.hook;
