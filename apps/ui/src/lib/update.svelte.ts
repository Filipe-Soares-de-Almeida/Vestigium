import { getVersion } from "@tauri-apps/api/app";
import { Channel, invoke } from "@tauri-apps/api/core";

import { lang } from "./i18n.svelte";

/** What the window remembers about updates; `check` is `null` until the first-open question is answered. */
export interface UpdatePrefs {
  check: boolean | null;
  last_check_ms: number | null;
  seen_version: string | null;
}

/** One heading of a version's notes ("Added", "Fixed") and its entries. */
export interface NotesGroup {
  kind: string;
  items: string[];
}

export interface Release {
  version: string;
  date: string | null;
  /** The paragraph before the version's first group, as one line. */
  summary: string | null;
  groups: NotesGroup[];
}

export type CheckResult =
  | { status: "up_to_date" }
  | { status: "available"; version: string; date_ms: number | null; summary: string | null; groups: NotesGroup[] };

type Available = Extract<CheckResult, { status: "available" }>;

export type Progress =
  | { kind: "chunk"; done: number; total: number | null }
  | { kind: "installing" }
  | { kind: "failed"; message: string };

const DAY_MS = 86_400_000;

/** The in-app update: the first-open question, the check, the download and the notes after updating. */
class Updates {
  version = $state("");
  prefs = $state<UpdatePrefs | null>(null);
  /** The one dialog on screen. */
  dialog = $state<"ask" | "available" | "news" | null>(null);
  found = $state.raw<Available | null>(null);
  /** The notes of the versions since the last one this window opened. */
  news = $state.raw<Release[]>([]);
  checking = $state(false);
  result = $state<"up_to_date" | "available" | "failed" | null>(null);
  download = $state<{ done: number; total: number | null } | null>(null);
  installing = $state(false);
  failed = $state(false);

  /** Runs once as the window opens: the notes after an update, then the question, then a daily check. */
  async start(): Promise<void> {
    try {
      [this.version, this.prefs] = await Promise.all([getVersion(), invoke<UpdatePrefs>("update_prefs")]);
    } catch {
      return;
    }
    const prefs = this.prefs;
    if (prefs.seen_version !== this.version) {
      const after = prefs.seen_version;
      await this.save({ seen_version: this.version });
      if (after !== null) {
        this.news = await invoke<Release[]>("release_notes", { lang: lang(), after }).catch(() => []);
        if (this.news.length) {
          this.dialog = "news";
          return;
        }
      }
    }
    if (prefs.check === null) {
      this.dialog = "ask";
      return;
    }
    if (prefs.check && (prefs.last_check_ms === null || Date.now() - prefs.last_check_ms >= DAY_MS)) {
      await this.check(true);
    }
  }

  /** The first-open answer; a yes checks at once. */
  async answer(yes: boolean): Promise<void> {
    this.dialog = null;
    await this.save({ check: yes });
    if (yes) await this.check(true);
  }

  async setCheck(on: boolean): Promise<void> {
    await this.save({ check: on });
  }

  /** Asks the release feed; a newer version opens its dialog. A daily check that fails stays quiet. */
  async check(auto = false): Promise<void> {
    if (this.checking) return;
    this.checking = true;
    try {
      const answer = await invoke<CheckResult>("update_check", { lang: lang() });
      if (this.prefs) this.prefs = { ...this.prefs, last_check_ms: Date.now() };
      if (answer.status === "available") {
        this.found = answer;
        this.result = "available";
        this.dialog ??= "available";
      } else {
        this.result = "up_to_date";
      }
    } catch {
      this.result = auto ? null : "failed";
    } finally {
      this.checking = false;
    }
  }

  /** Downloads the version found; at the end its installer takes over and the window closes. */
  async install(): Promise<void> {
    this.failed = false;
    this.download = { done: 0, total: null };
    const channel = new Channel<Progress>();
    channel.onmessage = (progress) => {
      if (progress.kind === "chunk") this.download = { done: progress.done, total: progress.total };
      else if (progress.kind === "installing") this.installing = true;
      else {
        this.download = null;
        this.installing = false;
        this.failed = true;
      }
    };
    try {
      await invoke("update_install", { progress: channel });
    } catch {
      this.download = null;
      this.failed = true;
    }
  }

  async cancel(): Promise<void> {
    await invoke("update_cancel").catch(() => undefined);
    this.download = null;
  }

  /** Closes the dialog; the question is asked again next time if it was not answered. */
  close(): void {
    if (this.download || this.installing) return;
    this.dialog = null;
    this.failed = false;
  }

  /** The version a check found, as a release with its day in local time. */
  get offered(): Release | null {
    const found = this.found;
    if (!found) return null;
    const day = found.date_ms === null ? null : new Date(found.date_ms);
    const date = day
      ? `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, "0")}-${String(day.getDate()).padStart(2, "0")}`
      : null;
    return { version: found.version, date, summary: found.summary, groups: found.groups };
  }

  /** Every released version up to this one, for Settings. */
  async allNotes(): Promise<Release[]> {
    return invoke<Release[]>("release_notes", { lang: lang(), after: null }).catch(() => []);
  }

  private async save(change: Partial<UpdatePrefs>): Promise<void> {
    const next = { ...(this.prefs ?? { check: null, last_check_ms: null, seen_version: null }), ...change };
    this.prefs = next;
    await invoke("set_update_prefs", { prefs: next }).catch(() => undefined);
  }
}

export const updates = new Updates();
