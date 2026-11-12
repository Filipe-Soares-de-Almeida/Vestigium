import { type AboutVersions, aboutVersions } from "./about";

/** The About dialog: open or closed, and what it last read about each program. */
class AboutDialog {
  open = $state(false);
  facts = $state.raw<AboutVersions | null>(null);
  failure = $state.raw<unknown>(null);

  /** Opens the dialog and reads the versions again, since the monitor may have changed since. */
  show(): void {
    this.open = true;
    void this.load();
  }

  close(): void {
    this.open = false;
  }

  async load(): Promise<void> {
    try {
      this.facts = await aboutVersions();
      this.failure = null;
    } catch (error) {
      this.failure = error;
    }
  }
}

export const aboutDialog = new AboutDialog();
