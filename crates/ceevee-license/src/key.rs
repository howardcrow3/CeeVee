//! License key shape and the `ceevee://activate` link from the purchase email.

use crate::{LicenseError, Result};

const ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Normalise what a person typed or pasted into `CEEV-XXXXX-XXXXX-XXXXX-XXXXX`.
/// Case, spaces and missing dashes are forgiven; the shape is checked, the key itself
/// is checked by the server.
pub fn normalize(input: &str) -> Result<String> {
    let flat: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();
    let body = flat
        .strip_prefix("CEEV")
        .ok_or(LicenseError::BadKeyFormat)?;
    // Crockford-style: people mistype O for 0 and I or L for 1.
    let body: String = body
        .chars()
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    if body.chars().count() != 20 || !body.chars().all(|c| ALPHABET.contains(c)) {
        return Err(LicenseError::BadKeyFormat);
    }
    let groups: Vec<&str> = (0..4).map(|i| &body[i * 5..i * 5 + 5]).collect();
    Ok(format!("CEEV-{}", groups.join("-")))
}

/// Last group only, for showing which key is active without printing all of it.
pub fn hint(key: &str) -> String {
    key.rsplit('-')
        .next()
        .map(|g| format!("CEEV-…-{g}"))
        .unwrap_or_default()
}

/// Pull the key out of `ceevee://activate?key=…`. Anything else is `None`.
pub fn parse_activation_link(url: &str) -> Option<String> {
    let rest = url.trim().strip_prefix("ceevee://activate")?;
    let query = rest.strip_prefix('?').or_else(|| rest.strip_prefix("/?"))?;
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("key="))
        .and_then(|v| normalize(&percent_decode(v)).ok())
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = [b[i + 1], b[i + 2]];
            if let Some(v) = std::str::from_utf8(&hex)
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: &str = "CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8";

    #[test]
    fn normalises_case_spaces_dashes_and_lookalikes() {
        assert_eq!(normalize(K).unwrap(), K);
        assert_eq!(normalize("  ceev 7kq2m 9xb4d tr5vw h3nz8 \n").unwrap(), K);
        assert_eq!(normalize("CEEV7KQ2M9XB4DTR5VWH3NZ8").unwrap(), K);
        assert_eq!(
            normalize("CEEV-OOOOO-IIIII-LLLLL-00000").unwrap(),
            "CEEV-00000-11111-11111-00000"
        );
    }

    #[test]
    fn rejects_wrong_shapes() {
        for bad in [
            "",
            "CEEV",
            "ABCD-7KQ2M-9XB4D-TR5VW-H3NZ8",
            "CEEV-7KQ2M-9XB4D-TR5VW",
            "CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8X",
            "CEEV-7KQ2M-9XB4D-TR5VW-H3NU!",
        ] {
            assert!(normalize(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn activation_link() {
        assert_eq!(
            parse_activation_link(&format!("ceevee://activate?key={K}")).as_deref(),
            Some(K)
        );
        assert_eq!(
            parse_activation_link("ceevee://activate/?utm=x&key=ceev%2D7kq2m9xb4dtr5vwh3nz8")
                .as_deref(),
            Some(K)
        );
        assert_eq!(parse_activation_link("ceevee://activate?key=nope"), None);
        assert_eq!(parse_activation_link("ceevee://activate"), None);
        assert_eq!(
            parse_activation_link("https://evil.example/activate?key=CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8"),
            None
        );
    }

    #[test]
    fn hint_hides_most_of_the_key() {
        assert_eq!(hint(K), "CEEV-…-H3NZ8");
    }
}
