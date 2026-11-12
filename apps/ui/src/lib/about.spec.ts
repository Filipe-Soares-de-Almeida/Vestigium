import { expect, test } from "vitest";

import en from "../../../../locales/en.json";
import { type AboutVersions, COMPONENTS, MAIN_ORDER, allRows, componentKey, copyBlock, licenseText, mainRows } from "./about";
import { setLang, t } from "./i18n.svelte";

test("licenses_show_as_declared_with_their_operators_in_words", () => {
  expect(licenseText("Apache-2.0 OR MIT")).toBe("Apache-2.0 or MIT");
  expect(licenseText("BSD-3-Clause AND MIT")).toBe("BSD-3-Clause and MIT");
  expect(licenseText("MIT/Apache-2.0")).toBe("MIT or Apache-2.0");
  expect(licenseText("Apache-2.0 / MIT")).toBe("Apache-2.0 or MIT");
  expect(licenseText("blessing")).toBe(t("ui.about.license.public_domain"));
  setLang("pt-BR");
  expect(licenseText("MIT OR Zlib OR Apache-2.0")).toBe("MIT ou Zlib ou Apache-2.0");
});

test("the_main_order_names_exactly_the_main_components_and_each_has_a_name", () => {
  const main = COMPONENTS.filter((c) => c.main).map((c) => c.name);
  expect([...MAIN_ORDER].sort()).toEqual([...main].sort());
  const catalog = en as Record<string, string>;
  for (const name of main) expect(catalog[componentKey(name)], name).toBeTruthy();
});

test("the_main_list_follows_the_credits_order_with_display_names", () => {
  const rows = mainRows();
  expect(rows.slice(0, 3).map((r) => r.name)).toEqual(["Tauri", "WRY", "TAO"]);
  expect(rows.find((r) => r.name === "uPlot")?.caption).toBe(t("ui.about.component.uplot.caption"));
  expect(rows.find((r) => r.name === "serde")?.caption).toBeNull();
});

test("the_full_list_keeps_declared_names_and_translates_only_rusts_own_library", () => {
  setLang("pt-BR");
  const names = allRows().map((r) => r.name);
  expect(names).toContain("tauri");
  expect(names).toContain("uplot");
  expect(names).toContain("Biblioteca padrão do Rust");
  expect(names).not.toContain("Rust standard library");
  expect(names).toHaveLength(COMPONENTS.length);
  expect(allRows().every((r) => r.caption === null)).toBe(true);
});

const FACTS: AboutVersions = {
  version: "0.1.1",
  released: "2026-11-11",
  monitor: { run: "running", version: "0.1.1", protocol: null },
  helper: "0.1.1",
  schema: 13,
  protocol: 22,
  windows: { edition: "Windows 11 Pro", family: "Windows 11", release: "24H2", build: "26100.2605", version: "10.0.26100.2605" },
  webview: "131.0.2903.112",
};

test("the_copied_block_is_english_markdown_of_what_the_versions_tab_shows", () => {
  setLang("pt-BR");
  expect(copyBlock(FACTS, { language: "System", detail_level: "Detailed" }, "pt-BR")).toBe(
    [
      "### Vestigium",
      "- Vestigium version: 0.1.1",
      "- Window: 0.1.1 (Vestigium.Window.exe)",
      "- Monitor: 0.1.1 (Vestigium.exe, running)",
      "- Administrator helper: 0.1.1 (Vestigium.Helper.exe)",
      "- History schema: 13",
      "- Pipe protocol: 22",
      "",
      "### Windows",
      "- Windows: Windows 11",
      "- Windows build: 10.0.26100.2605",
      "- Edition: Windows 11 Pro 24H2, 64-bit",
      "- WebView2: 131.0.2903.112",
      "",
      "### Settings",
      "- Language: Portuguese (Brazil), as Windows",
      "- Detail level: Detailed",
    ].join("\n"),
  );
});

test("the_copied_block_names_a_stale_monitor_and_leaves_out_what_was_not_read", () => {
  const bare = { ...FACTS, helper: null, windows: null, webview: null };
  const stale = copyBlock({ ...bare, monitor: { run: "other", version: null, protocol: 13 } }, null, "en");
  expect(stale).toContain("- Monitor: another version, protocol 13 (Vestigium.exe, running)");
  expect(stale).toContain("- Pipe protocol: 22 (monitor: 13)");
  expect(stale).toContain("- Administrator helper: not found (Vestigium.Helper.exe)");
  expect(stale).not.toContain("### Windows");
  expect(stale).toContain("### Settings\n- Language: English");
  expect(stale).not.toContain("Detail level");

  const stopped = copyBlock({ ...FACTS, monitor: { run: "stopped", version: "0.1.0", protocol: null } }, null, "en");
  expect(stopped).toContain("- Monitor: 0.1.0 (Vestigium.exe, not running)");
});
