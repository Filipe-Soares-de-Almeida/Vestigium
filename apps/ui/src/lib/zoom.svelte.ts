/** A stretch of a chart's axis, in whatever unit the chart uses. */
export interface Span {
  from: number;
  to: number;
}

export interface ZoomOptions {
  /** The whole stretch the chart can show. */
  bounds: () => Span;
  /** Width of the plot element in pixels. */
  width: () => number;
  /** The smallest stretch a zoomed view can cover. */
  minSpan: number;
  /** Pixels at each side of the plot element that the axis does not cover. */
  inset?: { left: number; right: number };
}

/** The zoomed view of a time chart: wheel, keys and drag; `view` is `null` while all of it shows. */
export class Zoom {
  view = $state<Span | null>(null);
  #options: ZoomOptions;
  #drag: { x: number; from: number; moved: boolean } | null = null;
  #dragged = false;

  constructor(options: ZoomOptions) {
    this.#options = options;
  }

  /** Start of the stretch on screen. */
  get from(): number {
    return this.view?.from ?? this.#options.bounds().from;
  }

  /** Length of the stretch on screen. */
  get span(): number {
    const { bounds } = this.#options;
    return this.view ? this.view.to - this.view.from : bounds().to - bounds().from;
  }

  /** Width of the axis in pixels. */
  get plotWidth(): number {
    const inset = this.#options.inset;
    return Math.max(1, this.#options.width() - (inset?.left ?? 0) - (inset?.right ?? 0));
  }

  /** The share of the axis, 0 to 1, under `px` pixels from the left of the plot element. */
  fraction(px: number): number {
    return Math.min(1, Math.max(0, (px - (this.#options.inset?.left ?? 0)) / this.plotWidth));
  }

  /** Zooms by `factor` around the position under `px`, staying inside the bounds. */
  zoom(factor: number, px: number): void {
    const bounds = this.#options.bounds();
    const frac = this.fraction(px);
    const at = this.from + frac * this.span;
    const next = Math.max(this.#options.minSpan, this.span * factor);
    if (next >= bounds.to - bounds.from) {
      this.view = null;
      return;
    }
    const from = Math.min(bounds.to - next, Math.max(bounds.from, at - frac * next));
    this.view = { from, to: from + next };
  }

  /** Moves a zoomed view so it starts at `from`, staying inside the bounds. */
  panTo(from: number): void {
    const view = this.view;
    if (!view) return;
    const bounds = this.#options.bounds();
    const span = view.to - view.from;
    const start = Math.min(bounds.to - span, Math.max(bounds.from, from));
    this.view = { from: start, to: start + span };
  }

  /** Moves a zoomed view the least that brings `at` inside it, centred when it was outside. */
  reveal(at: number): void {
    const view = this.view;
    if (view && (at < view.from || at >= view.to)) this.panTo(at - (view.to - view.from) / 2);
  }

  reset(): void {
    this.view = null;
  }

  /** Zooms by `factor` around the middle of what is on screen. */
  center(factor: number): void {
    this.zoom(factor, (this.#options.inset?.left ?? 0) + this.plotWidth / 2);
  }

  /** Handles + − and 0, zooming around `px`; `false` when the key is none of them. */
  key(key: string, px: number): boolean {
    if (key === "+" || key === "=") this.zoom(0.8, px);
    else if (key === "-" || key === "_") this.zoom(1.25, px);
    else if (key === "0") this.reset();
    else return false;
    return true;
  }

  /** Starts a drag that moves a zoomed view; the pointer stays on `el` until it is released. */
  press(event: PointerEvent, el: HTMLElement): void {
    if (!this.view || event.button !== 0) return;
    event.preventDefault();
    el.focus({ preventScroll: true });
    this.#drag = { x: event.clientX, from: this.view.from, moved: false };
    el.setPointerCapture(event.pointerId);
  }

  /** Moves the view with a drag; `true` while the drag has taken the pointer. */
  drag(event: PointerEvent): boolean {
    const drag = this.#drag;
    if (!drag) return false;
    const dx = event.clientX - drag.x;
    if (Math.abs(dx) > 3) drag.moved = true;
    if (!drag.moved) return false;
    this.panTo(drag.from - (dx / this.plotWidth) * this.span);
    return true;
  }

  release(): void {
    this.#dragged = this.#drag?.moved ?? false;
    this.#drag = null;
  }

  /** `true` once after a drag that moved the view, so the click that ends it can be ignored. */
  wasDragged(): boolean {
    const dragged = this.#dragged;
    this.#dragged = false;
    return dragged;
  }

  /** Ctrl and the wheel zoom around the pointer; Shift or a sideways wheel moves a zoomed view. */
  wheel(el: HTMLElement): () => void {
    const onwheel = (event: WheelEvent) => {
      if (event.ctrlKey) {
        event.preventDefault();
        this.zoom(event.deltaY < 0 ? 0.8 : 1.25, event.clientX - el.getBoundingClientRect().left);
      } else if (this.view && (event.shiftKey || Math.abs(event.deltaX) > Math.abs(event.deltaY))) {
        event.preventDefault();
        const delta = event.shiftKey ? event.deltaY : event.deltaX;
        this.panTo(this.from + (delta / this.plotWidth) * this.span);
      }
    };
    el.addEventListener("wheel", onwheel, { passive: false });
    return () => el.removeEventListener("wheel", onwheel);
  }
}

/** Action: wires the wheel of a chart's plot to its `Zoom`. */
export function wheelZoom(node: HTMLElement, zoom: Zoom): { destroy: () => void } {
  return { destroy: zoom.wheel(node) };
}
