//! How well a job fits, in words. Three states, never a number, and one plain sentence that says
//! why. The hard rules (remote only, lowest pay) are checked here before anything softer, so they
//! are never wrong and never left to a model.

use crate::categories::Criteria;
use crate::words;

/// Where a job stands for the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    Strong,
    Possible,
    Skip,
}

impl Fit {
    /// The stored score, 0 to 100, as one of three states.
    pub fn from_score(value: f64) -> Fit {
        if value >= 70.0 {
            Fit::Strong
        } else if value >= 40.0 {
            Fit::Possible
        } else {
            Fit::Skip
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Fit::Strong => "strong",
            Fit::Possible => "possible",
            Fit::Skip => "skip",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Fit::Strong => "Strong fit",
            Fit::Possible => "Possible fit",
            Fit::Skip => "Skip",
        }
    }
}

/// The top of the pay range in a posting's pay text, as dollars a year. Hourly pay is taken as a
/// 2,080 hour year. `None` when no figure can be read.
pub fn pay_ceiling(text: &str) -> Option<u32> {
    let lower = text.to_lowercase().replace("401k", "").replace("403b", "");
    let hourly = lower.contains("/hr")
        || lower.contains("/hour")
        || lower.contains("per hour")
        || lower.contains("an hour")
        || lower.contains("hourly");
    let chars: Vec<char> = lower.chars().collect();
    let mut best: Option<f64> = None;
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut num = String::new();
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == ',' || chars[i] == '.') {
            if chars[i] != ',' {
                num.push(chars[i]);
            }
            i += 1;
        }
        let Ok(mut v) = num.trim_end_matches('.').parse::<f64>() else {
            continue;
        };
        if i < chars.len() && chars[i] == 'k' {
            v *= 1000.0;
        }
        if hourly {
            v *= 2080.0;
        }
        best = Some(best.map_or(v, |b: f64| b.max(v)));
    }
    best.filter(|v| *v >= 1000.0).map(|v| v.round() as u32)
}

fn dollars(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("${out}")
}

/// A hard rule the posting breaks, in words, or `None` when it breaks none. Pay that is not listed
/// is never held against a job.
pub fn dealbreaker(
    c: &Criteria,
    title: &str,
    location: Option<&str>,
    pay: Option<&str>,
) -> Option<String> {
    if c.remote_only {
        let hay = format!("{} {}", words(title), words(location.unwrap_or("")));
        if !format!(" {hay} ").contains(" remote ") {
            return Some("not remote, and you only want remote".into());
        }
    }
    if c.min_pay > 0 {
        if let Some(top) = pay.and_then(pay_ceiling) {
            if top < c.min_pay {
                return Some(format!(
                    "pays up to {}, below your {} floor",
                    dollars(top),
                    dollars(c.min_pay)
                ));
            }
        }
    }
    None
}

/// Reasons that say what is missing rather than what matches.
fn is_gap(reason: &str) -> bool {
    reason.starts_with("does not mention")
        || reason.starts_with("not in a place")
        || reason.starts_with("title is not")
}

/// One sentence: the state, what matches, and what is missing. Built from the reasons the rule
/// scoring stored.
pub fn explain(value: f64, reasons: &[String]) -> String {
    let fit = Fit::from_score(value);
    let good: Vec<&str> = reasons
        .iter()
        .map(String::as_str)
        .filter(|r| !is_gap(r) && !r.contains("which you avoid"))
        .collect();
    let bad: Vec<&str> = reasons
        .iter()
        .map(String::as_str)
        .filter(|r| is_gap(r) || r.contains("which you avoid"))
        .collect();
    // A reason that rules the job out is the whole story.
    if fit == Fit::Skip && good.is_empty() && !bad.is_empty() {
        return format!("{}: {}.", fit.word(), bad.join("; "));
    }
    let mut out = fit.word().to_string();
    if !good.is_empty() {
        out.push_str(&format!(": {}", good.join(", ")));
    }
    if !bad.is_empty() {
        out.push_str(&format!(
            "{} {}",
            if good.is_empty() { ":" } else { ";" },
            bad.join(", ")
        ));
    }
    out.push('.');
    out
}

#[cfg(test)]
mod tests;
