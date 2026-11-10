//! The app's words in every language it speaks. The translations live in `locales/`, one folder
//! per language and one file per part of the app; `tr!` looks a key up in the language the
//! window is showing. The language is kept per thread: the window draws on one thread, and a
//! test that switches it cannot change what another test sees.
use std::cell::Cell;

use crate::enums::settings::Language;

thread_local! {
    static CURRENT: Cell<Language> = const { Cell::new(Language::English) };
}

/// The language the app is showing right now.
pub fn current() -> Language {
    CURRENT.with(Cell::get)
}

pub fn set_current(language: Language) {
    CURRENT.with(|current| current.set(language));
}

/// `text` in capitals. Greek capitals carry no accent marks, so `βαθμολογία` is `ΒΑΘΜΟΛΟΓΙΑ`,
/// where a plain `to_uppercase` would leave the accent on the Ι.
pub fn uppercase(text: &str) -> String {
    text.chars()
        .flat_map(|letter| match letter {
            'ά' => 'α'.to_uppercase(),
            'έ' => 'ε'.to_uppercase(),
            'ή' => 'η'.to_uppercase(),
            'ί' => 'ι'.to_uppercase(),
            'ό' => 'ο'.to_uppercase(),
            'ύ' => 'υ'.to_uppercase(),
            'ώ' => 'ω'.to_uppercase(),
            'ΐ' => 'ϊ'.to_uppercase(),
            'ΰ' => 'ϋ'.to_uppercase(),
            other => other.to_uppercase(),
        })
        .collect()
}

/// The words for a key in the current language, with `name = value` pairs filling its `%{name}`
/// holes. A key missing from Greek shows in English rather than as the bare key.
macro_rules! tr {
    ($key:expr $(, $name:ident = $value:expr)* $(,)?) => {
        ::rust_i18n::t!($key, locale = $crate::i18n::current().code() $(, $name = $value)*).to_string()
    };
}
pub(crate) use tr;

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs, path::Path};

    use super::{current, set_current, uppercase};
    use crate::enums::settings::Language;

    #[test]
    fn a_key_comes_back_in_the_language_being_shown() {
        set_current(Language::English);
        assert_eq!(tr!("settings.title"), "Settings");
        set_current(Language::Greek);
        assert_eq!(tr!("settings.title"), "Ρυθμίσεις");
        set_current(Language::English);
    }

    #[test]
    fn greek_capitals_drop_their_accents_and_english_ones_are_plain() {
        assert_eq!(uppercase("βαθμολογία"), "ΒΑΘΜΟΛΟΓΙΑ");
        assert_eq!(uppercase("Παρτίδες ανάλυση ώρα"), "ΠΑΡΤΙΔΕΣ ΑΝΑΛΥΣΗ ΩΡΑ");
        assert_eq!(uppercase("ΐ"), "Ϊ");
        assert_eq!(uppercase("Rating"), "RATING");
    }

    #[test]
    fn the_language_starts_in_english_on_every_thread() {
        set_current(Language::Greek);
        let other = std::thread::spawn(current).join().expect("the thread ends");
        assert_eq!(other, Language::English);
        set_current(Language::English);
    }

    /// Every key of every file in `locales/` for one language, by its dotted path.
    fn messages(language: Language) -> BTreeMap<String, String> {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        let mut messages = BTreeMap::new();
        for area in fs::read_dir(folder).expect("the locales folder reads") {
            let file = area
                .expect("an area reads")
                .path()
                .join(format!("{}.yml", language.code()));
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|_| panic!("{} exists for every area", file.display()));
            let tree: serde_json::Value =
                serde_saphyr::from_str(&text).expect("the file is valid YAML");
            flatten("", &tree, &mut messages);
        }
        messages
    }

    fn flatten(prefix: &str, node: &serde_json::Value, into: &mut BTreeMap<String, String>) {
        match node {
            serde_json::Value::Object(children) => {
                for (name, child) in children {
                    let key = if prefix.is_empty() {
                        name.clone()
                    } else {
                        format!("{prefix}.{name}")
                    };
                    flatten(&key, child, into);
                }
            }
            serde_json::Value::String(text) => {
                assert!(
                    into.insert(prefix.to_owned(), text.clone()).is_none(),
                    "{prefix} twice"
                );
            }
            other => panic!("{prefix} is {other}, not text"),
        }
    }

    /// The `%{name}` holes in a message, in sorted order.
    fn holes(message: &str) -> Vec<&str> {
        let mut holes: Vec<&str> = message
            .split("%{")
            .skip(1)
            .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
            .collect();
        holes.sort_unstable();
        holes
    }

    #[test]
    fn english_and_greek_have_the_same_keys_and_holes() {
        let (english, greek) = (messages(Language::English), messages(Language::Greek));
        let missing_in_greek: Vec<_> = english
            .keys()
            .filter(|key| !greek.contains_key(*key))
            .collect();
        let missing_in_english: Vec<_> = greek
            .keys()
            .filter(|key| !english.contains_key(*key))
            .collect();
        assert!(
            missing_in_greek.is_empty(),
            "not in Greek: {missing_in_greek:?}"
        );
        assert!(
            missing_in_english.is_empty(),
            "not in English: {missing_in_english:?}"
        );
        for (key, text) in &english {
            assert_eq!(
                holes(text),
                holes(&greek[key]),
                "{key} fills different holes"
            );
        }
    }

    #[test]
    fn no_message_is_empty() {
        for language in Language::ALL {
            for (key, text) in messages(language) {
                assert!(
                    !text.trim().is_empty(),
                    "{key} is empty in {}",
                    language.code()
                );
            }
        }
    }
}
