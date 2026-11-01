// Settings in the browser: the version and the data folder. `?data=none` has no folder yet;
// `?fail=set_settings:refused` turns every save down, as the watcher does with an invalid setting.
import type { Handler } from "../mock";

const query = new URLSearchParams(location.search);

export const handlers: Record<string, Handler> = {
  about: () => ({
    version: "0.0.0",
    data_bytes: query.get("data") === "none" ? null : 96 * 1024 * 1024 + 314_159,
  }),
  open_data_folder: () => null,
};
