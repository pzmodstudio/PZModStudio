//! The app's help pages (`docs/app/<lang>/<page>.md`), written for the player and bundled into
//! the exe: the window shows them ("?" on each screen), and MCP clients read them
//! (`read_app_doc`) to answer questions about the Studio the way it really works.

macro_rules! pages {
    ($($name:literal),* $(,)?) => {
        /// (page, English, Spanish)
        pub const PAGES: &[(&str, &str, &str)] = &[
            $(($name,
               include_str!(concat!("../../docs/app/en/", $name, ".md")),
               include_str!(concat!("../../docs/app/es/", $name, ".md")))),*
        ];
    };
}

pages!("index", "profiles", "saves", "monitor", "mod-list", "whats-new", "health", "browse", "maps", "merger", "developer", "servers", "settings");

/// A page in a language ("es", else English); None for a page that is not there.
pub fn page(name: &str, language: &str) -> Option<&'static str> {
    let name = name.trim().trim_end_matches(".md");
    // (a byte-order mark an editor left at the start would hide the title)
    PAGES.iter().find(|(n, _, _)| *n == name).map(|(_, en, es)| if language.starts_with("es") { *es } else { *en }).map(|t| t.trim_start_matches('\u{feff}'))
}

/// Each page's name and its title (the first line).
pub fn list() -> Vec<(&'static str, String)> {
    PAGES.iter().map(|(n, en, _)| (*n, en.lines().next().unwrap_or("").trim_start_matches('#').trim().to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_is_there_in_both_languages_with_a_title() {
        for (name, en, es) in PAGES {
            assert!(en.starts_with("# "), "{} (en) starts with its title", name);
            assert!(es.starts_with("# "), "{} (es) starts with its title", name);
            for text in [en, es] {
                assert!(!text.contains("<table") && !text.contains("](http"), "{}: the help renderer shows no tables or links", name);
            }
        }
        assert!(page("mod-list", "es").unwrap().starts_with("# Lista de mods"));
        assert!(page("mod-list.md", "en").is_some());
        assert!(page("nope", "en").is_none());
        assert_eq!(list().len(), PAGES.len());
    }
}
