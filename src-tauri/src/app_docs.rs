//! The app's help pages (`docs/app/<lang>/<page>.md`), written for the player and bundled into
//! the exe: the window shows them ("?" on each screen), and MCP clients read them
//! (`read_app_doc`) to answer questions about the Studio the way it really works.

use std::collections::BTreeSet;

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

/// A section of the guide that answers a question.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DocHit {
    pub page: String,
    pub heading: String,
    pub text: String,
    pub score: f32,
}

fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' => 'a',
            'é' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'ï' => 'i',
            'ó' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'ü' => 'u',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// Words that say nothing about the topic.
const STOP: &[&str] = &[
    "que", "como", "para", "por", "con", "una", "uno", "los", "las", "del", "el", "la", "de", "en", "es", "se", "lo", "le", "mi", "mis", "un", "y", "o", "a",
    "hago", "hacer", "puedo", "sirve", "donde", "esta", "este", "eso", "the", "how", "what", "where", "does", "do", "can", "is", "a", "an", "of", "to", "in", "on",
    "for", "and", "or", "my", "i", "it", "app", "studio",
];

fn terms(s: &str) -> Vec<String> {
    fold(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !STOP.contains(w))
        // a stem: "perfiles" and "perfil" meet
        .map(|w| w.chars().take(6).collect())
        .collect()
}

/// The sections (a `##`/`###` heading and its text) that share the most words with the question,
/// a word in the heading counting three times.
pub fn search(query: &str, language: &str, max: usize) -> Vec<DocHit> {
    let want: BTreeSet<String> = terms(query).into_iter().collect();
    if want.is_empty() {
        return vec![];
    }
    // the question's words in a row ("compatible copy", "copia compatible"): a section that says
    // them together is about that, not one that names each word somewhere
    let phrase: String = fold(query).split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3 && !STOP.contains(w)).collect::<Vec<_>>().join(" ");
    let together = |text: &str| phrase.contains(' ') && fold(text).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ").contains(&phrase);
    let mut hits = Vec::new();
    for (name, en, es) in PAGES {
        let text = if language.starts_with("es") { es } else { en };
        let mut heading = text.lines().next().unwrap_or("").trim_start_matches('#').trim().to_string();
        let mut body = String::new();
        let flush = |heading: &str, body: &str, hits: &mut Vec<DocHit>| {
            if body.trim().is_empty() {
                return;
            }
            let h: BTreeSet<String> = terms(heading).into_iter().collect();
            let b: BTreeSet<String> = terms(body).into_iter().collect();
            let mut score = want.iter().map(|w| if h.contains(w) { 3.0 } else if b.contains(w) { 1.0 } else { 0.0 }).sum::<f32>() / want.len() as f32;
            if together(heading) {
                score += 4.0;
            } else if together(body) {
                score += 1.5;
            }
            if score > 0.0 {
                // the paragraph (or list item) that answers best, not the whole section: the one
                // with the question's words together, else the most of them
                let para_score = |p: &str| {
                    let t: BTreeSet<String> = terms(p).into_iter().collect();
                    want.iter().filter(|w| t.contains(*w)).count() as f32 + if together(p) { 3.0 } else { 0.0 }
                };
                let paragraphs: Vec<&str> = body.split("\n\n").flat_map(|p| if p.lines().all(|l| l.trim_start().starts_with("- ")) { p.lines().collect::<Vec<_>>() } else { vec![p] }).filter(|p| !p.trim().is_empty()).collect();
                let best = paragraphs.iter().copied().max_by(|a, b| para_score(a).partial_cmp(&para_score(b)).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(body);
                // a section whose best paragraph says the words together answers best
                if together(best) {
                    score += 1.0;
                }
                hits.push(DocHit { page: name.to_string(), heading: heading.to_string(), text: best.trim().to_string(), score });
            }
        };
        for line in text.lines().skip(1) {
            if line.starts_with("## ") || line.starts_with("### ") {
                flush(&heading, &body, &mut hits);
                heading = line.trim_start_matches('#').trim().to_string();
                body.clear();
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }
        flush(&heading, &body, &mut hits);
    }
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(max);
    hits
}

/// Each page's name and its title (the first line).
pub fn list() -> Vec<(&'static str, String)> {
    PAGES.iter().map(|(n, en, _)| (*n, en.lines().next().unwrap_or("").trim_start_matches('#').trim().to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_answers_a_question_with_its_section() {
        let hits = search("¿cómo hago un perfil nuevo?", "es", 3);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].page, "profiles", "{:?}", hits.iter().map(|h| (&h.page, &h.heading)).collect::<Vec<_>>());
        let hits = search("what is a compatible copy", "en", 3);
        assert!(hits.iter().any(|h| h.text.to_lowercase().contains("compatible copy")));
    }

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
