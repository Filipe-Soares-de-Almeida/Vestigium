//! UI text catalogs for Argos.

use std::collections::HashMap;

const EN: &str = include_str!("../../../locales/en.json");
const PT_BR: &str = include_str!("../../../locales/pt-BR.json");

/// A language the UI ships in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
}

impl Lang {
    /// Maps a Windows LANGID; every Portuguese variant gets pt-BR, everything else English.
    pub fn from_langid(langid: u16) -> Lang {
        const LANG_PORTUGUESE: u16 = 0x16;
        if langid & 0x3FF == LANG_PORTUGUESE {
            Lang::PtBr
        } else {
            Lang::En
        }
    }
}

/// UI text of one language, falling back to English for a missing key.
#[derive(Clone, Debug)]
pub struct Catalog {
    text: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Catalog {
    pub fn load(lang: Lang) -> Catalog {
        match lang {
            Lang::En => Catalog::from_maps(parse(EN), HashMap::new()),
            Lang::PtBr => Catalog::from_maps(parse(PT_BR), parse(EN)),
        }
    }

    pub fn from_maps(text: HashMap<String, String>, fallback: HashMap<String, String>) -> Catalog {
        Catalog { text, fallback }
    }

    /// The text for `key`: this language, then English, then the key itself.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.text
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map_or(key, String::as_str)
    }

    /// The text for `key` with each `{name}` replaced by its value.
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        args.iter()
            .fold(self.get(key).to_string(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            })
    }

    /// Keys this language translates itself, without the fallback.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.text.keys().map(String::as_str)
    }
}

fn parse(json: &str) -> HashMap<String, String> {
    serde_json::from_str(json).expect("embedded catalog is a flat JSON object of strings")
}
