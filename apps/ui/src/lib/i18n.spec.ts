import { expect, test } from "vitest";

import { setLang, t, tn } from "./i18n.svelte";

test("t_fills_the_named_placeholders", () => {
  const text = t("ui.now.all.count", { shown: 5, total: 1200 });
  expect(text).toContain("5");
  expect(text).toContain("1,200");
  expect(text).not.toContain("{");
});

test("t_leaves_a_placeholder_without_a_value_visible", () => {
  expect(t("ui.now.all.count", { shown: 5 })).toContain("{total}");
});

test("t_formats_numbers_and_picks_text_for_the_selected_language", () => {
  const english = t("ui.common.retry");
  setLang("pt-BR");
  expect(t("ui.now.all.count", { shown: 5, total: 1200 })).toContain("1.200");
  expect(t("ui.common.retry")).not.toBe(english);
  setLang("en");
  expect(document.documentElement.lang).toBe("en");
});

test("tn_picks_the_plural_form_of_the_language", () => {
  const key = "ui.shell.nav.tweaks.signals";
  expect(tn(key, 1)).toBe(t(`${key}.one`, { count: 1 }));
  expect(tn(key, 3)).toBe(t(`${key}.other`, { count: 3 }));
  expect(tn(key, 1)).not.toBe(tn(key, 3));
});
