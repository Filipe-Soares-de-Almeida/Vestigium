import { agentStatus, setPaused, setSettings, settings, startAgent, timeline } from "./api";
import { errorKind, errorText } from "./errors";
import { t } from "./i18n.svelte";
import { applySettings } from "./prefs";
import { toast } from "./toast.svelte";
import type { AgentLink, HistoryState, Settings, SettingsDto } from "./types";
import { pollWhileShown } from "./visibility";

const POLL_MS = 5000;
const PROBE_MS = 60_000;

/** What every screen reads about the watcher: running, paused or stopped, its settings, the history. */
class App {
  /** `null` until the first answer. */
  link = $state<AgentLink | null>(null);
  /** `null` when the running watcher could not be asked. */
  settings = $state<SettingsDto | null>(null);
  history = $state<HistoryState | null>(null);
  /** Why the window could not start, when it could not. */
  failure = $state<unknown>(null);
  starting = $state(false);
  startError = $state<unknown>(null);

  #languageSet = false;

  // Flags the screens read; each one changes only when its value does.
  loading = $derived(this.failure === null && (this.link === null || this.history === null));
  running = $derived(this.link?.link === "running");
  paused = $derived(this.link?.link === "running" && this.link.paused);
  /** When the pause ends by itself; `null` for a pause held until resumed. */
  pausedUntil = $derived(this.link?.link === "running" && this.link.paused ? (this.link.paused_until_ms ?? null) : null);
  stopped = $derived(this.link?.link === "stopped");
  /** A watcher answers on the pipe but not as this window expects. */
  unreachable = $derived(this.link?.link === "mismatch");
  /** Sensitive mode saw stutters worth a deep capture. */
  suggestCapture = $derived(this.link?.link === "running" && this.link.suggest_capture);
  /** Settings and pause can change only while the watcher runs. */
  editable = $derived(this.settings?.editable ?? false);

  /** Loads everything, then keeps the watcher's state current; returns the function that stops it. */
  start(): () => void {
    void this.load();
    return pollWhileShown(() => this.refresh(), POLL_MS);
  }

  async load(): Promise<void> {
    this.failure = null;
    let [link, history] = await Promise.allSettled([agentStatus(), probeHistory()]);
    if (link.status === "fulfilled") this.link = link.value;
    // A history from an older version is unreadable until the watcher updates it: start the watcher if it
    // is stopped, then ask again for a few seconds while it does.
    let started = false;
    for (let attempt = 0; attempt < 6 && history.status === "rejected" && errorKind(history.reason) === "store"; attempt++) {
      if (this.link?.link === "stopped" && !started) {
        started = true;
        await this.startWatcher();
      } else {
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      [history] = await Promise.allSettled([probeHistory()]);
    }
    try {
      if (link.status === "rejected") throw link.reason;
      if (history.status === "rejected") throw history.reason;
      this.history = history.value;
      // With no history at all this is the first launch, so monitoring starts without waiting for a click.
      if (this.link?.link === "stopped" && this.history === "empty") await this.startWatcher();
      await this.loadSettings();
    } catch (error) {
      this.failure = error;
    }
  }

  /** Asks the watcher again; settings are reloaded when it started, stopped or came back. */
  async refresh(): Promise<void> {
    if (this.link === null) return;
    try {
      const before = this.link.link;
      this.link = await agentStatus();
      if (this.history === "empty") this.history = await probeHistory();
      if (this.link.link !== before) await this.loadSettings();
    } catch {
      // The next poll asks again; the window keeps what it last knew.
    }
  }

  /** Saves settings through the watcher and applies their appearance at once. */
  async saveSettings(next: Settings): Promise<void> {
    await setSettings(next);
    if (this.settings) this.settings = { ...this.settings, settings: next };
    applySettings(next, { language: false });
  }

  async resume(): Promise<void> {
    try {
      await setPaused(false);
      await this.refresh();
      toast.show(t("ui.shell.toast.resumed"));
    } catch (error) {
      toast.show(errorText(error));
    }
  }

  async startWatcher(): Promise<void> {
    this.starting = true;
    this.startError = null;
    try {
      await startAgent();
      await this.refresh();
      toast.show(t("ui.shell.toast.started"));
    } catch (error) {
      this.startError = error;
    } finally {
      this.starting = false;
    }
  }

  async loadSettings(): Promise<void> {
    try {
      this.settings = await settings();
    } catch (error) {
      if (this.link?.link !== "mismatch") throw error;
      this.settings = null;
      return;
    }
    applySettings(this.settings.settings, { language: !this.#languageSet });
    this.#languageSet = true;
  }
}

/** Whether the store holds any history at all, read from a one-point timeline. */
async function probeHistory(): Promise<HistoryState> {
  const now = Date.now();
  return (await timeline(now - PROBE_MS, now, 1)).history;
}

export const app = new App();
