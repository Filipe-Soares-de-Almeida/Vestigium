import { agentStatus, setPaused, setSettings, settings, startAgent, timeline } from "./api";
import { errorKind, errorText } from "./errors";
import { t } from "./i18n.svelte";
import { applySettings } from "./prefs";
import { toast } from "./toast.svelte";
import type { AgentLink, HistoryState, Settings, SettingsDto } from "./types";
import { pollWhileShown } from "./visibility";
import { unverifiedSince } from "./monitor";

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
  /** The refresh under way, so polls and clicks never ask twice at once; `#queued` asks once more after it. */
  #refreshing: Promise<void> | null = null;
  #queued = false;

  // Flags the screens read; each one changes only when its value does.
  loading = $derived(this.failure === null && (this.link === null || this.history === null));
  running = $derived(this.link?.link === "running");
  paused = $derived(this.link?.link === "running" && this.link.paused);
  /** When the pause ends by itself; `null` for a pause held until resumed. */
  pausedUntil = $derived(this.link?.link === "running" && this.link.paused ? (this.link.paused_until_ms ?? null) : null);
  stopped = $derived(this.link?.link === "stopped");
  /** A watcher answers on the pipe but not as this window expects. */
  unreachable = $derived(this.link?.link === "mismatch");
  /** The watcher owns the pipe but did not answer in time; the history on disk still reads. */
  silent = $derived(this.link?.link === "silent");
  /** How long the window waited, in seconds, while the watcher is silent. */
  silentSeconds = $derived(this.link?.link === "silent" ? this.link.seconds : 0);
  /** Sensitive mode saw stutters worth a deep capture. */
  suggestCapture = $derived(this.link?.link === "running" && this.link.suggest_capture);
  /** The watcher's account of its own watching; `null` unless it runs. */
  monitor = $derived(this.link?.link === "running" ? this.link.monitor : null);
  /** When the watcher started, Unix milliseconds; now while it does not run. */
  started = $derived(this.link?.link === "running" ? this.link.started_at_ms : Date.now());
  /** The last thing that went wrong in the watcher, for the technical level. */
  lastError = $derived(this.link?.link === "running" ? this.link.last_error : null);
  /** The watcher runs but has produced no sample for a while, and nothing is paused or asleep. */
  noReading = $derived(this.monitor?.state === "no_reading");
  /** Samples come, but some data source fails. */
  degraded = $derived(this.monitor?.state === "degraded");
  /** Since when the health check has not finished, once that has lasted long enough; the poll refreshes it. */
  unverified = $derived(
    this.link?.link === "running" && !this.paused && !this.link.suspended ? unverifiedSince(this.link, Date.now()) : null,
  );
  /** Settings and pause can change only while the watcher runs. */
  editable = $derived(this.settings?.editable ?? false);

  /** Loads everything, then keeps the watcher's state current; returns the function that stops it. */
  start(): () => void {
    void this.load();
    return pollWhileShown(() => this.refresh(), POLL_MS);
  }

  async load(): Promise<void> {
    this.failure = null;
    let [link, history] = await Promise.allSettled([askStatus(), probeHistory()]);
    if (link.status === "fulfilled" && link.value !== null) this.link = link.value;
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
      if (link.value === null) throw new Error("the watcher is busy");
      if (history.status === "rejected") throw history.reason;
      this.history = history.value;
      // With no history at all this is the first launch, so monitoring starts without waiting for a click.
      if (this.link?.link === "stopped" && this.history === "empty") await this.startWatcher();
      await this.loadSettings();
    } catch (error) {
      this.failure = error;
    }
  }

  /**
   * Asks the watcher again; settings are reloaded when it started, stopped, went silent or came back.
   * A call while one is under way waits for it and then asks once more, so what it returns is current.
   */
  refresh(): Promise<void> {
    if (this.link === null) return Promise.resolve();
    if (this.#refreshing) {
      this.#queued = true;
      return this.#refreshing;
    }
    this.#refreshing = (async () => {
      try {
        do {
          this.#queued = false;
          await this.#ask();
        } while (this.#queued);
      } finally {
        this.#refreshing = null;
      }
    })();
    return this.#refreshing;
  }

  async #ask(): Promise<void> {
    if (this.link === null) return;
    try {
      const before = this.link.link;
      const next = await askStatus();
      if (next === null) return;
      this.link = next;
      if (this.history === "empty") this.history = await probeHistory();
      if (next.link !== before) await this.loadSettings();
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
      if (this.link?.link !== "mismatch" && this.link?.link !== "silent") throw error;
      this.settings = null;
      return;
    }
    applySettings(this.settings.settings, { language: !this.#languageSet });
    this.#languageSet = true;
  }
}

/** The watcher's state; one more try when another command held the connection, `null` if it still does. */
async function askStatus(): Promise<AgentLink | null> {
  const first = await agentStatus();
  if (first.link !== "busy") return first;
  await new Promise((resolve) => setTimeout(resolve, 300));
  const second = await agentStatus();
  return second.link === "busy" ? null : second;
}

/** Whether the store holds any history at all, read from a one-point timeline. */
async function probeHistory(): Promise<HistoryState> {
  const now = Date.now();
  return (await timeline(now - PROBE_MS, now, 1)).history;
}

export const app = new App();
