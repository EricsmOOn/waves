use crate::config::LocaleTable;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Catalog {
    active_locale: String,
    default_locale: String,
    locales: HashMap<String, LocaleTable>,
}

impl Catalog {
    pub fn new(
        active_locale: impl Into<String>,
        default_locale: impl Into<String>,
        locales: HashMap<String, LocaleTable>,
    ) -> Self {
        Self {
            active_locale: active_locale.into(),
            default_locale: default_locale.into(),
            locales,
        }
    }

    pub fn active_locale(&self) -> &str {
        &self.active_locale
    }

    pub fn text(&self, key: &str) -> String {
        self.lookup(key).unwrap_or(key).to_string()
    }

    pub fn format(&self, key: &str, vars: &[(&str, String)]) -> String {
        let mut text = self.text(key);
        for (name, value) in vars {
            text = text.replace(&format!("{{{}}}", name), value);
        }
        text
    }

    fn lookup(&self, key: &str) -> Option<&str> {
        self.locales
            .get(&self.active_locale)
            .and_then(|table| table.entries.get(key))
            .or_else(|| {
                self.locales
                    .get(&self.default_locale)
                    .and_then(|table| table.entries.get(key))
            })
            .map(|value| value.as_str())
    }
}
