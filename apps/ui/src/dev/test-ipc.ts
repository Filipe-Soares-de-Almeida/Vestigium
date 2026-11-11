import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";

export type Handler = (args: Record<string, unknown>) => unknown;

/** The commands a test answers, by name; each returns a value, a promise or throws. */
export type Handlers = Record<string, Handler>;

/** Every command the window sent, in order. */
export interface Calls {
  all: { cmd: string; args: Record<string, unknown> }[];
  /** The arguments of each call to `cmd`. */
  of(cmd: string): Record<string, unknown>[];
}

/** Answers the window's commands from `handlers`; any other command rejects with its name. */
export function answer(handlers: Handlers): Calls {
  const all: Calls["all"] = [];
  mockIPC(async (cmd, args) => {
    all.push({ cmd, args: (args ?? {}) as Record<string, unknown> });
    const handler = handlers[cmd];
    if (!handler) throw new Error(`unexpected command: ${cmd}`);
    return handler((args ?? {}) as Record<string, unknown>);
  });
  return { all, of: (cmd) => all.filter((call) => call.cmd === cmd).map((call) => call.args) };
}

/** A rejection shaped like the window's `UiError`. */
export function refusal(kind: string): { kind: string; message: string } {
  return { kind, message: `test: ${kind}` };
}

export function resetIpc(): void {
  clearMocks();
}
