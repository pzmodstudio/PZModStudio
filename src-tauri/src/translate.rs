//! Mod descriptions in the player's language, on demand.
//!
//! The text is Steam's BBCode: tags, line breaks and runs without letters are kept as they are,
//! only the words between them go to the service. Every text translated is kept on disk, so a
//! description is sent once. Services, in order: DeepL when the player saved a key, else
//! Google's free endpoint (no account; unofficial, so it may stop one day), else MyMemory for
//! the pieces whose language the script gives away.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DEEPL_KEY: &str = "deepl_api_key";

/// What came back for a list of texts, in the same order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub texts: Vec<String>,
    /// The language the service found the text in ("ru"), or "" when it did not say.
    pub source: String,
    /// "deepl", "google" or "mymemory"; "cache" when every text had been translated before.
    pub service: String,
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent(format!("PZModStudio/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// The language a text's script gives away: Cyrillic, kana, hangul, Han, Thai, Arabic. None for
/// the Latin alphabet (English, Spanish, German... look alike here) and for mixed texts where
/// Latin letters dominate.
pub fn script_language(text: &str) -> Option<&'static str> {
    let (mut cyr, mut kana, mut hangul, mut han, mut thai, mut arabic, mut latin) = (0, 0, 0, 0, 0, 0, 0);
    for c in text.chars() {
        match c as u32 {
            0x0400..=0x04FF => cyr += 1,
            0x3040..=0x30FF => kana += 1,
            0xAC00..=0xD7AF | 0x1100..=0x11FF => hangul += 1,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF => han += 1,
            0x0E00..=0x0E7F => thai += 1,
            0x0600..=0x06FF => arabic += 1,
            0x41..=0x5A | 0x61..=0x7A => latin += 1,
            _ => {}
        }
    }
    let other = cyr + kana + hangul + han + thai + arabic;
    if other == 0 || other * 4 < latin {
        return None;
    }
    // The script with most letters wins: a Russian description that quotes one Chinese name is
    // Russian. Japanese mixes kana and Han, so they count together when there is kana enough.
    let japanese = if kana > 0 && kana * 10 >= han { kana + han } else { 0 };
    let chinese = if japanese > 0 { 0 } else { han };
    [("ja", japanese), ("ko", hangul), ("zh", chinese), ("ru", cyr), ("th", thai), ("ar", arabic)]
        .into_iter()
        .fold(None, |best: Option<(&'static str, usize)>, x| if x.1 > best.map(|b| b.1).unwrap_or(0) { Some(x) } else { best })
        .map(|b| b.0)
}

/// A text as pieces: `(piece, goes to the service)`. BBCode tags, line breaks and runs with no
/// letter stay as they are.
fn segments(text: &str) -> Vec<(String, bool)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, bool)> = Vec::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut Vec<(String, bool)>| {
        if !run.is_empty() {
            let letters = run.chars().any(char::is_alphabetic);
            out.push((std::mem::take(run), letters));
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '[' {
            if let Some(end) = chars[i..].iter().position(|&x| x == ']').filter(|&e| e < 80) {
                flush(&mut run, &mut out);
                out.push((chars[i..=i + end].iter().collect(), false));
                i += end + 1;
                continue;
            }
        }
        if c == '\n' || c == '\r' {
            flush(&mut run, &mut out);
            out.push((c.to_string(), false));
            i += 1;
            continue;
        }
        run.push(c);
        i += 1;
    }
    flush(&mut run, &mut out);
    out
}

/// A URL in the middle of a sentence is sent as is; a piece that is only a URL is not sent.
fn is_url(piece: &str) -> bool {
    let t = piece.trim();
    (t.starts_with("http://") || t.starts_with("https://")) && !t.contains(' ')
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Google's free endpoint: the pieces joined by line breaks, the answer split by them. The
/// detected language comes back too.
fn google(agent: &ureq::Agent, pieces: &[String], target: &str) -> Result<(Vec<String>, String), String> {
    let q = pieces.join("\n");
    let url = format!("https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&ie=UTF-8&oe=UTF-8", percent_encode(target));
    let mut resp = agent
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(format!("q={}", percent_encode(&q)).as_bytes())
        .map_err(|e| format!("Google did not answer: {}", e))?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(format!("Google answered {}", status));
    }
    let v: serde_json::Value = resp.body_mut().read_json().map_err(|e| format!("Google's answer is not JSON: {}", e))?;
    let joined: String = v
        .get(0)
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|s| s.get(0).and_then(|t| t.as_str())).collect())
        .unwrap_or_default();
    let source = v.get(2).and_then(|s| s.as_str()).unwrap_or("").to_string();
    let parts: Vec<String> = joined.split('\n').map(|s| s.trim_end_matches('\r').to_string()).collect();
    if parts.len() != pieces.len() {
        return Err(format!("Google gave {} lines for {}", parts.len(), pieces.len()));
    }
    Ok((parts, source))
}

/// MyMemory takes up to 500 bytes a call, and a few thousand words a day for free.
const MYMEMORY_BYTES: usize = 480;
const MYMEMORY_CALLS: usize = 25;

/// MyMemory, one piece at a time, only when the script says the language (it needs one).
fn mymemory(agent: &ureq::Agent, piece: &str, source: &str, target: &str) -> Result<String, String> {
    let url = format!(
        "https://api.mymemory.translated.net/get?q={}&langpair={}|{}",
        percent_encode(piece),
        percent_encode(source),
        percent_encode(target)
    );
    let mut resp = agent.get(&url).call().map_err(|e| format!("MyMemory did not answer: {}", e))?;
    let v: serde_json::Value = resp.body_mut().read_json().map_err(|e| format!("MyMemory's answer is not JSON: {}", e))?;
    let status = v.get("responseStatus").and_then(|s| s.as_i64()).unwrap_or(0);
    if status != 200 {
        return Err(format!("MyMemory answered {}: {}", status, v.get("responseDetails").and_then(|d| d.as_str()).unwrap_or("")));
    }
    v.get("responseData")
        .and_then(|d| d.get("translatedText"))
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .ok_or_else(|| "MyMemory gave no text".to_string())
}

fn deepl_host(key: &str) -> &'static str {
    if key.ends_with(":fx") { "https://api-free.deepl.com" } else { "https://api.deepl.com" }
}

/// DeepL, every piece in one call.
fn deepl(agent: &ureq::Agent, key: &str, pieces: &[String], target: &str) -> Result<(Vec<String>, String), String> {
    let mut form: Vec<(&str, String)> = pieces.iter().map(|p| ("text", p.clone())).collect();
    form.push(("target_lang", target.to_uppercase()));
    let body: String = form.iter().map(|(k, v)| format!("{}={}", k, percent_encode(v))).collect::<Vec<_>>().join("&");
    let mut resp = agent
        .post(&format!("{}/v2/translate", deepl_host(key)))
        .header("Authorization", &format!("DeepL-Auth-Key {}", key))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send(body.as_bytes())
        .map_err(|e| format!("DeepL did not answer: {}", e))?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(format!("DeepL answered {}", status));
    }
    let v: serde_json::Value = resp.body_mut().read_json().map_err(|e| format!("DeepL's answer is not JSON: {}", e))?;
    let list = v.get("translations").and_then(|t| t.as_array()).ok_or("DeepL gave no translations")?;
    let texts: Vec<String> = list.iter().map(|t| t.get("text").and_then(|s| s.as_str()).unwrap_or("").to_string()).collect();
    if texts.len() != pieces.len() {
        return Err(format!("DeepL gave {} texts for {}", texts.len(), pieces.len()));
    }
    let source = list.first().and_then(|t| t.get("detected_source_language")).and_then(|s| s.as_str()).unwrap_or("").to_lowercase();
    Ok((texts, source))
}

/// Checks a DeepL key against its usage endpoint and keeps it.
pub fn save_deepl_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.len() < 20 {
        return Err("A DeepL key is longer than that.".to_string());
    }
    let resp = agent()
        .get(&format!("{}/v2/usage", deepl_host(key)))
        .header("Authorization", &format!("DeepL-Auth-Key {}", key))
        .call()
        .map_err(|e| format!("DeepL did not answer: {}", e))?;
    match resp.status().as_u16() {
        200 => crate::secrets::set(DEEPL_KEY, key),
        403 => Err("DeepL does not know this key.".to_string()),
        s => Err(format!("DeepL answered {}", s)),
    }
}

/// The translations kept on disk: how many and how big.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub count: usize,
    pub bytes: u64,
}

fn cache_dir(user_zomboid_dir: &str) -> Option<PathBuf> {
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next()?;
    Some(z.join("PZModStudio_Cache").join("translate"))
}

pub fn cache_info(user_zomboid_dir: &str) -> CacheInfo {
    let mut info = CacheInfo { count: 0, bytes: 0 };
    let Some(dir) = cache_dir(user_zomboid_dir) else { return info };
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.path().extension().map(|x| x == "json").unwrap_or(false) {
            info.count += 1;
            info.bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    info
}

/// Deletes every kept translation; the next request goes out again. Returns how many went.
pub fn clear_cache(user_zomboid_dir: &str) -> Result<usize, String> {
    let before = cache_info(user_zomboid_dir).count;
    if let Some(dir) = cache_dir(user_zomboid_dir) {
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("Could not delete {}: {}", dir.display(), e))?;
        }
    }
    Ok(before)
}

/// Where translations are kept: `<Zomboid>/PZModStudio_Cache/translate/<hash>.json`.
fn cache_file(user_zomboid_dir: &str, target: &str, text: &str) -> Option<PathBuf> {
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next()?;
    let mut h = Sha256::new();
    h.update(target.as_bytes());
    h.update([0]);
    h.update(text.as_bytes());
    let hash = format!("{:x}", h.finalize());
    Some(z.join("PZModStudio_Cache").join("translate").join(format!("{}.json", &hash[..32])))
}

#[derive(Serialize, Deserialize)]
struct Kept {
    text: String,
    source: String,
    service: String,
}

/// The pieces of several texts translated by whichever service is available, as one list.
fn translate_pieces(user_zomboid_dir: &str, pieces: &[String], source_hint: Option<&str>, target: &str) -> Result<(Vec<String>, String, String), String> {
    let _ = user_zomboid_dir;
    let agent = agent();
    let deepl_key = crate::secrets::get(DEEPL_KEY).ok().flatten();
    let mut errors: Vec<String> = Vec::new();
    if let Some(key) = deepl_key.as_deref() {
        // DeepL takes up to 50 texts and 128 KiB a call.
        let mut out: Vec<String> = Vec::with_capacity(pieces.len());
        let mut source = String::new();
        let mut ok = true;
        for chunk in pieces.chunks(50) {
            match deepl(&agent, key, chunk, target) {
                Ok((texts, src)) => {
                    out.extend(texts);
                    if source.is_empty() {
                        source = src;
                    }
                }
                Err(e) => {
                    errors.push(e);
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            return Ok((out, source, "deepl".to_string()));
        }
    }
    // Google: batches of about 1800 characters, pieces joined by line breaks; a batch whose
    // answer does not split back goes piece by piece.
    let mut out: Vec<String> = Vec::with_capacity(pieces.len());
    let mut source = String::new();
    let mut google_failed = false;
    let mut batch: Vec<String> = Vec::new();
    let mut size = 0usize;
    let mut batches: Vec<Vec<String>> = Vec::new();
    for p in pieces {
        if size + p.chars().count() > 1800 && !batch.is_empty() {
            batches.push(std::mem::take(&mut batch));
            size = 0;
        }
        size += p.chars().count();
        batch.push(p.clone());
    }
    if !batch.is_empty() {
        batches.push(batch);
    }
    'batches: for b in &batches {
        match google(&agent, b, target) {
            Ok((texts, src)) => {
                out.extend(texts);
                if source.is_empty() {
                    source = src;
                }
            }
            // Refused for too many requests: every other call is refused too, so go on at once.
            Err(first) if first.ends_with(" 429") => {
                errors.push(first);
                google_failed = true;
                break 'batches;
            }
            Err(first) => {
                for p in b {
                    match google(&agent, std::slice::from_ref(p), target) {
                        Ok((texts, src)) => {
                            out.extend(texts);
                            if source.is_empty() {
                                source = src;
                            }
                        }
                        Err(e) => {
                            errors.push(first.clone());
                            errors.push(e);
                            google_failed = true;
                            break 'batches;
                        }
                    }
                }
            }
        }
    }
    if !google_failed {
        return Ok((out, source, "google".to_string()));
    }
    // MyMemory, when the caller or the script says what language it is.
    let whole = pieces.join("\n");
    if let Some(src) = source_hint.or_else(|| script_language(&whole)) {
        // Pieces joined by line breaks up to MyMemory's 500 bytes a call; a call whose answer
        // does not split back goes piece by piece. Counted first: a description that needs more
        // calls than the free daily share is refused at once, not after 40 calls.
        let mut calls: Vec<Vec<&String>> = Vec::new();
        for p in pieces {
            match calls.last_mut() {
                Some(c) if c.iter().map(|x| x.len() + 1).sum::<usize>() + p.len() <= MYMEMORY_BYTES => c.push(p),
                _ => calls.push(vec![p]),
            }
        }
        if calls.len() > MYMEMORY_CALLS {
            errors.push(format!("MyMemory: this text needs {} calls, more than the {} a translation may use", calls.len(), MYMEMORY_CALLS));
        } else {
            let mut out: Vec<String> = Vec::with_capacity(pieces.len());
            for c in &calls {
                let joined = c.iter().map(|x| x.as_str()).collect::<Vec<_>>().join("\n");
                let got = mymemory(&agent, &joined, src, target).map_err(|e| format!("{}; then {}", errors.join("; "), e))?;
                let parts: Vec<String> = got.lines().map(str::to_string).collect();
                if parts.len() == c.len() {
                    out.extend(parts);
                } else {
                    for p in c {
                        out.push(mymemory(&agent, p, src, target).map_err(|e| format!("{}; then {}", errors.join("; "), e))?);
                    }
                }
            }
            return Ok((out, src.to_string(), "mymemory".to_string()));
        }
    }
    Err(format!("No translation service answered: {}", errors.join("; ")))
}

/// `texts` in `target` ("es"), in the same order; the ones translated before come from disk.
pub fn translate_texts(user_zomboid_dir: &str, texts: &[String], target: &str) -> Result<Translation, String> {
    translate_texts_from(user_zomboid_dir, texts, None, target)
}

/// The same, told what language the texts are in ("en"), for the fallback service that needs it.
pub fn translate_texts_from(user_zomboid_dir: &str, texts: &[String], source_hint: Option<&str>, target: &str) -> Result<Translation, String> {
    let target = target.trim().to_lowercase();
    if target.len() < 2 {
        return Err("No target language.".to_string());
    }
    let mut out: Vec<Option<String>> = vec![None; texts.len()];
    let mut source = String::new();
    let mut service = "cache".to_string();
    // From disk first.
    for (i, t) in texts.iter().enumerate() {
        if t.trim().is_empty() {
            out[i] = Some(t.clone());
            continue;
        }
        if let Some(kept) = cache_file(user_zomboid_dir, &target, t)
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str::<Kept>(&s).ok())
        {
            if source.is_empty() {
                source = kept.source;
            }
            out[i] = Some(kept.text);
        }
    }
    // The rest, as one list of pieces.
    let todo: Vec<usize> = (0..texts.len()).filter(|&i| out[i].is_none()).collect();
    if !todo.is_empty() {
        let mut layout: Vec<(usize, Vec<(String, bool)>)> = Vec::new();
        let mut pieces: Vec<String> = Vec::new();
        for &i in &todo {
            let segs = segments(&texts[i]);
            for (piece, send) in &segs {
                if *send && !is_url(piece) {
                    pieces.push(piece.clone());
                }
            }
            layout.push((i, segs));
        }
        let (translated, src, used) = if pieces.is_empty() {
            (Vec::new(), String::new(), "cache".to_string())
        } else {
            translate_pieces(user_zomboid_dir, &pieces, source_hint, &target)?
        };
        if source.is_empty() {
            source = src.clone();
        }
        service = used.clone();
        let mut k = 0;
        for (i, segs) in layout {
            let mut text = String::new();
            for (piece, send) in segs {
                if send && !is_url(&piece) {
                    // Keep the piece's own leading and trailing blanks: the service trims them.
                    let lead: String = piece.chars().take_while(|c| c.is_whitespace()).collect();
                    let trail: String = piece.chars().rev().take_while(|c| c.is_whitespace()).collect::<Vec<_>>().into_iter().rev().collect();
                    text.push_str(&lead);
                    text.push_str(translated.get(k).map(|s| s.trim()).unwrap_or(""));
                    text.push_str(&trail);
                    k += 1;
                } else {
                    text.push_str(&piece);
                }
            }
            if let Some(f) = cache_file(user_zomboid_dir, &target, &texts[i]) {
                if let Some(dir) = f.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let kept = Kept { text: text.clone(), source: src.clone(), service: used.clone() };
                let _ = std::fs::write(f, serde_json::to_string(&kept).unwrap_or_default());
            }
            out[i] = Some(text);
        }
    }
    // A service names the language of the whole batch and often says "en" for a Chinese text
    // with a few English words in it; the script is surer when it says something.
    let whole = texts.join("\n");
    if let Some(lang) = script_language(&whole) {
        source = lang.to_string();
    }
    Ok(Translation { texts: out.into_iter().map(Option::unwrap_or_default).collect(), source, service })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_script_names_the_language_and_latin_does_not() {
        assert_eq!(script_language("Мод добавляет новые предметы"), Some("ru"));
        assert_eq!(script_language("这个模组添加了新物品"), Some("zh"));
        assert_eq!(script_language("Плавные объёмные модели для вида от первого лица, автор 中文"), Some("ru"), "a few Han letters in Russian text");
        assert_eq!(script_language("Плавные объёмные модели の"), Some("ru"));
        assert_eq!(script_language("このMODは新しいアイテムを追加します"), Some("ja"));
        assert_eq!(script_language("새로운 아이템을 추가합니다"), Some("ko"));
        assert_eq!(script_language("Adds new items to the game"), None);
        // A mostly English text with a Chinese line is still sent as "unknown": the service decides.
        assert_eq!(script_language("Adds new items to the game, with recipes and a full set of textures and sounds 新物品"), None);
    }

    #[test]
    fn tags_breaks_and_urls_stay_and_only_words_go() {
        let segs = segments("[h1]Заголовок[/h1]\nТекст [url=https://x.y]ссылка[/url]\nhttps://example.com\n[b][/b] 123");
        let sent: Vec<&str> = segs.iter().filter(|(p, s)| *s && !is_url(p)).map(|(p, _)| p.as_str()).collect();
        assert_eq!(sent, vec!["Заголовок", "Текст ", "ссылка"]);
        let kept: String = segs.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(kept, "[h1]Заголовок[/h1]\nТекст [url=https://x.y]ссылка[/url]\nhttps://example.com\n[b][/b] 123");
    }

    /// Needs the network: `cargo test -- --ignored google_answers`.
    #[test]
    #[ignore]
    fn google_answers_a_russian_bbcode_text() {
        let dir = std::env::temp_dir().join(format!("pzms_translate_{}", std::process::id()));
        let text = "[h1]Описание[/h1]
Мод добавляет новые предметы в игру.
[url=https://example.com]Ссылка[/url]".to_string();
        let out = translate_texts(dir.to_str().unwrap(), &[text], "es").expect("a translation");
        println!("{:?}", out);
        assert_eq!(out.service, "google");
        assert_eq!(out.source, "ru");
        assert!(out.texts[0].starts_with("[h1]") && out.texts[0].contains("[/h1]
") && out.texts[0].contains("[url=https://example.com]"));
        assert!(!out.texts[0].contains("Описание"));
    }

    #[test]
    fn the_form_is_percent_encoded() {
        assert_eq!(percent_encode("a b&c=д"), "a%20b%26c%3D%D0%B4");
    }
}
