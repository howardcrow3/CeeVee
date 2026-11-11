//! The fact guard (TECHNICAL_PLAN section 5). After any drafting task, code checks that the
//! numbers, names and acronyms in the output appear in what the model was given: the profile
//! and the posting. Anything else is flagged, and the draft is not saved until the user decides.
//!
//! This is a deliberately plain check, not a language model: it errs toward flagging, because a
//! false flag costs the user a click and a missed invention puts a false claim in an application.
//!
//! What it checks:
//! - **Numbers**: every number with digits (years, percentages, money, counts).
//! - **Names**: capitalized words in the middle of a sentence, and acronyms such as SQL or AWS.
//!   Words that start a sentence are skipped, as are common letter words (Dear, Team, Sincerely).

use std::collections::BTreeSet;

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlagKind {
    Number,
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Flag {
    pub kind: FlagKind,
    pub text: String,
    /// A few words around the first place it appears, for the review screen.
    pub context: String,
}

/// Capitalized words that appear in letters without being facts.
const COMMON: &[&str] = &[
    "dear",
    "hello",
    "hi",
    "sincerely",
    "regards",
    "best",
    "thanks",
    "thank",
    "yours",
    "team",
    "hiring",
    "manager",
    "committee",
    "recruiter",
    "sir",
    "madam",
    "mr",
    "ms",
    "mrs",
    "dr",
    "i",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

struct Token {
    text: String,
    sentence_start: bool,
    start: usize,
    end: usize,
}

fn tokens(s: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut at_start = true;
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() {
        let c = s[i..].chars().next().unwrap();
        if c.is_alphanumeric() {
            let start = i;
            let mut end = i;
            for (off, ch) in s[i..].char_indices() {
                // Keep letters and digits, and joiners that sit between them (3.5, 1,000, can't, co-op).
                let inner = matches!(ch, '.' | ',' | '\'' | '’' | '-' | '/')
                    && s[i + off + ch.len_utf8()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric)
                    && s[..i + off]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_alphanumeric);
                if ch.is_alphanumeric() || inner {
                    end = i + off + ch.len_utf8();
                } else {
                    break;
                }
            }
            out.push(Token {
                text: s[start..end].to_owned(),
                sentence_start: at_start,
                start,
                end,
            });
            at_start = false;
            i = end;
        } else {
            if matches!(c, '.' | '!' | '?' | '\n' | ':') {
                at_start = true;
            }
            i += c.len_utf8();
        }
    }
    out
}

fn is_number(t: &str) -> bool {
    t.chars().any(|c| c.is_ascii_digit())
        && t.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '/' | '-'))
}

fn norm_number(t: &str) -> String {
    t.chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .trim_end_matches('.')
        .to_owned()
}

/// The parts of a number token that must appear in the sources. A range like 2018-2021 or 40-50 has
/// two parts. In a year-month such as 2021-03 the month is not checked, because sources spell months
/// as words ("March 2021").
fn number_parts(t: &str) -> Vec<String> {
    let parts: Vec<&str> = t.split(['-', '/']).filter(|p| !p.is_empty()).collect();
    let mut out = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let month_after_year = i > 0 && p.len() == 2 && parts[i - 1].len() == 4;
        if !month_after_year {
            out.push(norm_number(p));
        }
    }
    out
}

fn word_key(t: &str) -> String {
    let t = t.trim_end_matches("'s").trim_end_matches("’s");
    t.to_lowercase()
}

fn is_name_like(t: &str) -> bool {
    let mut chars = t.chars();
    let first = chars.next().unwrap_or(' ');
    if !first.is_uppercase() || is_number(t) {
        return false;
    }
    // A lone capital letter ("I", "A") is not a name.
    t.chars().count() > 1
}

fn is_acronym(t: &str) -> bool {
    t.chars().count() >= 2
        && t.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())
        && t.chars().any(char::is_alphabetic)
}

fn context(s: &str, tok: &Token) -> String {
    let before: Vec<&str> = s[..tok.start].split_whitespace().rev().take(3).collect();
    let after: Vec<&str> = s[tok.end..].split_whitespace().take(3).collect();
    let mut parts: Vec<&str> = before.into_iter().rev().collect();
    parts.push(&s[tok.start..tok.end]);
    parts.extend(after);
    parts.join(" ")
}

struct Known {
    words: BTreeSet<String>,
    numbers: BTreeSet<String>,
}

fn index(sources: &[&str]) -> Known {
    let mut k = Known {
        words: BTreeSet::new(),
        numbers: BTreeSet::new(),
    };
    for s in sources {
        for t in tokens(s) {
            if is_number(&t.text) {
                k.numbers.extend(number_parts(&t.text));
            } else {
                k.words.insert(word_key(&t.text));
            }
        }
    }
    k
}

/// Scan one piece of text. With `short_value` the first word counts as mid-sentence, because a
/// short field such as a skill or a company name is not a sentence.
fn scan(
    output: &str,
    known: &Known,
    short_value: bool,
    seen: &mut BTreeSet<(FlagKind, String)>,
    flags: &mut Vec<Flag>,
) {
    for (i, t) in tokens(output).into_iter().enumerate() {
        let at_start = t.sentence_start && !(short_value && i == 0);
        let flag = if is_number(&t.text) {
            let unknown = number_parts(&t.text)
                .iter()
                .any(|n| !known.numbers.contains(n));
            unknown.then_some(FlagKind::Number)
        } else if is_acronym(&t.text) {
            (!known.words.contains(&word_key(&t.text))).then_some(FlagKind::Name)
        } else if is_name_like(&t.text) && !at_start {
            let key = word_key(&t.text);
            (!COMMON.contains(&key.as_str()) && !known.words.contains(&key))
                .then_some(FlagKind::Name)
        } else {
            None
        };
        if let Some(kind) = flag {
            if seen.insert((kind, t.text.to_lowercase())) {
                flags.push(Flag {
                    kind,
                    context: context(output, &t),
                    text: t.text.clone(),
                });
            }
        }
    }
}

/// Check `output` against the texts it was allowed to draw from. Returns one flag per distinct
/// number or name that the sources do not contain, in order of appearance.
pub fn check(output: &str, sources: &[&str]) -> Vec<Flag> {
    let known = index(sources);
    let (mut seen, mut flags) = (BTreeSet::new(), Vec::new());
    scan(output, &known, false, &mut seen, &mut flags);
    flags
}

/// Check every string and number inside a JSON value, such as a resume. Short values (a skill, a
/// company, a title) are checked word by word including their first word; longer ones, such as
/// bullet points, are treated as sentences.
pub fn check_value(value: &Value, sources: &[&str]) -> Vec<Flag> {
    fn walk(
        v: &Value,
        known: &Known,
        seen: &mut BTreeSet<(FlagKind, String)>,
        flags: &mut Vec<Flag>,
    ) {
        match v {
            Value::String(s) => scan(s, known, s.split_whitespace().count() <= 4, seen, flags),
            Value::Number(n) => scan(&n.to_string(), known, true, seen, flags),
            Value::Array(a) => a.iter().for_each(|x| walk(x, known, seen, flags)),
            Value::Object(o) => o.values().for_each(|x| walk(x, known, seen, flags)),
            _ => {}
        }
    }
    let known = index(sources);
    let (mut seen, mut flags) = (BTreeSet::new(), Vec::new());
    walk(value, &known, &mut seen, &mut flags);
    flags
}

#[cfg(test)]
mod tests;
