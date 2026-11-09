import en from "../../../../locales/en.json";
import ptBR from "../../../../locales/pt-BR.json";

import type { LanguagePref } from "./types";

export type Lang = "en" | "pt-BR";

type Args = Record<string, string | number>;

const catalogs: Record<Lang, Record<string, string>> = { en, "pt-BR": ptBR };

/** Every Portuguese variant gets pt-BR, everything else English. */
export function pickLang(language: string): Lang {
  return language.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

/** The language a setting asks for; `System` follows Windows. */
export function langFor(pref: LanguagePref): Lang {
  if (pref === "En") return "en";
  if (pref === "PtBr") return "pt-BR";
  return pickLang(navigator.language);
}

let current = $state<Lang>(pickLang(navigator.language));

export function lang(): Lang {
  return current;
}

export function setLang(next: Lang): void {
  current = next;
  document.documentElement.lang = next;
}

/** The text for `key`, with each `{name}` replaced: this language, then English, then the key itself. */
export function t(key: string, args?: Args): string {
  const text = catalogs[current][key] ?? catalogs.en[key] ?? key;
  if (!args) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in args ? format(args[name]) : whole,
  );
}

/** `key.one`, `key.other` and so on, picked by the language's plural rule, with `{count}` filled in. */
export function tn(key: string, count: number, args?: Args): string {
  const rule = new Intl.PluralRules(current).select(count);
  const chosen = `${key}.${rule}` in catalogs[current] ? `${key}.${rule}` : `${key}.other`;
  return t(chosen, { count, ...args });
}

function format(value: string | number): string {
  return typeof value === "number" ? new Intl.NumberFormat(current).format(value) : value;
}
