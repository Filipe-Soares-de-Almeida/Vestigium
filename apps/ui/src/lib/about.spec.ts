import { expect, test } from "vitest";

import en from "../../../../locales/en.json";
import { COMPONENTS, MAIN_ORDER, allRows, componentKey, licenseText, mainRows } from "./about";
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
