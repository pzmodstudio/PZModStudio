//! The app's help pages (`docs/app/<lang>/<page>.md`), written for the player and bundled into
//! the exe: the window shows them ("?" on each screen), and MCP clients read them
//! (`read_app_doc`) to answer questions about the Studio the way it really works.

use std::collections::BTreeSet;

/// The languages the help is written in, in the order of each page's texts.
pub const LANGUAGES: [&str; 7] = ["en", "es", "pt-BR", "fr", "de", "ru", "zh-CN"];

macro_rules! pages {
    ($($name:literal),* $(,)?) => {
        /// (page, its text in each of LANGUAGES)
        pub const PAGES: &[(&str, [&str; 7])] = &[
            $(($name, [
               include_str!(concat!("../../docs/app/en/", $name, ".md")),
               include_str!(concat!("../../docs/app/es/", $name, ".md")),
               include_str!(concat!("../../docs/app/pt-BR/", $name, ".md")),
               include_str!(concat!("../../docs/app/fr/", $name, ".md")),
               include_str!(concat!("../../docs/app/de/", $name, ".md")),
               include_str!(concat!("../../docs/app/ru/", $name, ".md")),
               include_str!(concat!("../../docs/app/zh-CN/", $name, ".md")),
            ])),*
        ];
    };
}

pages!("index", "profiles", "saves", "monitor", "mod-list", "whats-new", "health", "browse", "maps", "merger", "developer", "servers", "settings");

/// Which of LANGUAGES a language tag reads: the exact one ("pt-BR"), else the same first part
/// ("es-AR" -> es, "pt" -> pt-BR, "zh" -> zh-CN), else English.
fn language_index(language: &str) -> usize {
    let lower = language.trim().to_lowercase();
    if let Some(i) = LANGUAGES.iter().position(|l| l.to_lowercase() == lower) {
        return i;
    }
    let base = lower.split(['-', '_']).next().unwrap_or("");
    LANGUAGES.iter().position(|l| l.to_lowercase().split('-').next() == Some(base)).unwrap_or(0)
}

/// A page's text in a language (English when not written in it).
fn text_in(texts: &[&'static str; 7], language: &str) -> &'static str {
    texts[language_index(language)]
}

/// A page in a language (else English); None for a page that is not there.
pub fn page(name: &str, language: &str) -> Option<&'static str> {
    let name = name.trim().trim_end_matches(".md");
    // (a byte-order mark an editor left at the start would hide the title)
    PAGES.iter().find(|(n, _)| *n == name).map(|(_, texts)| text_in(texts, language)).map(|t| t.trim_start_matches('\u{feff}'))
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
            'â' | 'ã' => 'a',
            'ê' => 'e',
            'î' => 'i',
            'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            'ё' => 'е',
            'й' => 'и',
            c => c,
        })
        .collect()
}

/// Words that say nothing about the topic.
const STOP: &[&str] = &[
    "que", "como", "para", "por", "con", "una", "uno", "los", "las", "del", "el", "la", "de", "en", "es", "se", "lo", "le", "mi", "mis", "un", "y", "o", "a",
    "hago", "hacer", "puedo", "sirve", "donde", "esta", "este", "eso", "the", "how", "what", "where", "does", "do", "can", "is", "a", "an", "of", "to", "in", "on",
    "for", "and", "or", "my", "i", "it", "app", "studio",
    "fazer", "faco", "uma", "onde", "posso", "com", "comment", "faire", "est", "une", "les", "des", "quoi", "pour", "avec", "dans", "wie", "was",
    "ist", "ein", "eine", "der", "die", "das", "und", "mache", "kann", "ich", "mit", "von", "как", "что", "это", "где", "мне", "сделать", "для", "или",
];

/// Chinese words that say nothing about the topic (two characters, as Chinese is cut).
const STOP_ZH: &[&str] = &["怎么", "什么", "如何", "哪里", "可以", "一个", "我的", "这个", "是什", "么是"];

fn is_han(c: char) -> bool {
    matches!(c, '㐀'..='鿿')
}

fn terms(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for w in fold(s).split(|c: char| !c.is_alphanumeric()) {
        if w.chars().any(is_han) {
            // Chinese has no spaces: each two characters in a row are a term, and the letters
            // around them ("MCP 服务器") a word of their own
            let chars: Vec<char> = w.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let han = is_han(chars[i]);
                let start = i;
                while i < chars.len() && is_han(chars[i]) == han {
                    i += 1;
                }
                let run = &chars[start..i];
                if han {
                    if run.len() == 1 {
                        out.push(run[0].to_string());
                    }
                    for pair in run.windows(2) {
                        let t: String = pair.iter().collect();
                        if !STOP_ZH.contains(&t.as_str()) {
                            out.push(t);
                        }
                    }
                } else if run.len() >= 3 {
                    out.push(run.iter().take(6).collect());
                }
            }
        } else if w.chars().count() >= 3 && !STOP.contains(&w) {
            // a stem: "perfiles" and "perfil" meet
            out.push(w.chars().take(6).collect());
        }
    }
    out
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
    let phrase: String = fold(query).split(|c: char| !c.is_alphanumeric()).filter(|w| w.chars().count() >= 3 && !STOP.contains(w)).collect::<Vec<_>>().join(" ");
    let together = |text: &str| phrase.contains(' ') && fold(text).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ").contains(&phrase);
    let mut hits = Vec::new();
    for (name, texts) in PAGES {
        let text = text_in(texts, language);
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
    PAGES.iter().map(|(n, texts)| (*n, texts[0].lines().next().unwrap_or("").trim_start_matches('#').trim().to_string())).collect()
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
    fn the_guide_answers_in_every_language() {
        for (question, lang) in [("como faço um perfil novo?", "pt-BR"), ("comment créer un profil ?", "fr"), ("wie erstelle ich ein neues Profil?", "de"), ("как создать новый профиль?", "ru"), ("怎么创建新的配置方案？", "zh-CN")] {
            let hits = search(question, lang, 3);
            assert!(!hits.is_empty(), "{lang}: no answer");
            assert_eq!(hits[0].page, "profiles", "{lang}: {:?}", hits.iter().map(|h| (&h.page, &h.heading)).collect::<Vec<_>>());
        }
    }

    #[test]
    fn every_page_is_there_in_every_language_with_a_title() {
        for (name, texts) in PAGES {
            for (lang, text) in LANGUAGES.iter().zip(texts.iter()) {
                assert!(text.trim_start_matches('﻿').starts_with("# "), "{} ({}) starts with its title", name, lang);
                assert!(!text.contains("<table") && !text.contains("](http"), "{} ({}): the help renderer shows no tables or links", name, lang);
            }
        }
        assert!(page("settings", "pt-BR").unwrap() != page("settings", "en").unwrap());
        assert_eq!(page("settings", "pt"), page("settings", "pt-BR"));
        assert_eq!(page("settings", "es-AR"), page("settings", "es"));
        assert_eq!(page("settings", "ja"), page("settings", "en"));
        assert!(page("mod-list", "es").unwrap().starts_with("# Lista de mods"));
        assert!(page("mod-list.md", "en").is_some());
        assert!(page("nope", "en").is_none());
        assert_eq!(list().len(), PAGES.len());
    }
}
