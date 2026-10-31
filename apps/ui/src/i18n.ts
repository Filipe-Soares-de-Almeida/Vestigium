import en from "../../../locales/en.json";
import ptBR from "../../../locales/pt-BR.json";

export type Lang = "en" | "pt-BR";

const catalogs: Record<Lang, Record<string, string>> = { en, "pt-BR": ptBR };

/** Every Portuguese variant gets pt-BR, everything else English. */
export function pickLang(language: string): Lang {
  return language.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const lang: Lang = pickLang(navigator.language);

/** The text for `key`: this language, then English, then the key itself. */
export function t(key: string): string {
  return catalogs[lang][key] ?? catalogs.en[key] ?? key;
}
