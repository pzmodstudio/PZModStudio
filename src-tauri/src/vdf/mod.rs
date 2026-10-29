//! Minimal reader for Valve's text KeyValues format (.acf / .vdf files written by Steam):
//! `"key" "value"` pairs and `"key" { ... }` blocks, `//` comments, `\"` / `\\` escapes.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Value(String),
    Block(BTreeMap<String, Vdf>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Block(map) => map.get(key).or_else(|| map.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)),
            Vdf::Value(_) => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Vdf::Value(v) => Some(v),
            Vdf::Block(_) => None,
        }
    }

    pub fn u64(&self, key: &str) -> Option<u64> {
        self.str(key)?.trim().parse().ok()
    }

    pub fn block(&self) -> Option<&BTreeMap<String, Vdf>> {
        match self {
            Vdf::Block(map) => Some(map),
            Vdf::Value(_) => None,
        }
    }
}

enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => {}
                        },
                        '"' => break,
                        _ => s.push(c),
                    }
                }
                tokens.push(Token::Str(s));
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                // Unquoted token (rare in Steam files).
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || n == '{' || n == '}' || n == '"' {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                tokens.push(Token::Str(s));
            }
        }
    }
    tokens
}

fn parse_block(tokens: &[Token], pos: &mut usize) -> BTreeMap<String, Vdf> {
    let mut map = BTreeMap::new();
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Close => {
                *pos += 1;
                break;
            }
            Token::Open => {
                *pos += 1; // stray brace: skip
            }
            Token::Str(key) => {
                let key = key.clone();
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Open) => {
                        *pos += 1;
                        map.insert(key, Vdf::Block(parse_block(tokens, pos)));
                    }
                    Some(Token::Str(value)) => {
                        map.insert(key, Vdf::Value(value.clone()));
                        *pos += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    map
}

/// Parses a whole file; the result is a block holding the root key(s).
pub fn parse(text: &str) -> Vdf {
    let tokens = tokenize(text);
    let mut pos = 0;
    Vdf::Block(parse_block(&tokens, &mut pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_steam_acf() {
        let text = r#""AppWorkshop"
{
	"appid"		"108600"
	// comment
	"WorkshopItemsInstalled"
	{
		"2447729538"
		{
			"size"		"9447432"
			"timeupdated"		"1786359514"
		}
	}
	"Name"	"quoted \"name\" here"
}"#;
        let root = parse(text);
        let app = root.get("AppWorkshop").unwrap();
        assert_eq!(app.u64("appid"), Some(108600));
        let item = app.get("WorkshopItemsInstalled").unwrap().get("2447729538").unwrap();
        assert_eq!(item.u64("timeupdated"), Some(1786359514));
        assert_eq!(app.str("Name"), Some("quoted \"name\" here"));
        assert_eq!(app.get("workshopitemsinstalled").map(|b| b.block().unwrap().len()), Some(1), "keys are case-insensitive");
    }
}
