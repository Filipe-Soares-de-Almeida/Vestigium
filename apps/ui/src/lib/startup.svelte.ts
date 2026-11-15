import { startupArea } from "./api";
import type { StartupAreaDto } from "./types";

/** What the startup area and the page of one startup both read, so moving between them shows what was read before. */
class Startup {
  /** `null` until the first read. */
  dto = $state.raw<StartupAreaDto | null>(null);
  error = $state<unknown>(null);

  /** A failed read keeps what is on screen; the error shows only when there is nothing yet. */
  async load(): Promise<void> {
    if (this.dto === null) this.error = null;
    try {
      this.dto = await startupArea();
      this.error = null;
    } catch (error) {
      if (this.dto === null) this.error = error;
    }
  }
}

export const startup = new Startup();
