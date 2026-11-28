//! A calendar hold for an interview that an email announced. The time is the one the email
//! gave, with no time zone, so the calendar app shows it at the same clock time wherever the
//! person is. Nothing is sent anywhere.

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// `YYYY-MM-DDTHH:MM` plus `minutes`, as the compact local form calendars use (`20261203T170000`).
/// `None` when `at` is not in that form.
fn compact(at: &str, minutes: i64) -> Option<String> {
    let b = at.as_bytes();
    if b.len() < 16 || b[10] != b'T' {
        return None;
    }
    let n = |a: usize, z: usize| at.get(a..z).and_then(|s| s.parse::<i64>().ok());
    let (mut y, mut mo, mut d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (mut h, mut mi) = (n(11, 13)?, n(14, 16)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 {
        return None;
    }
    mi += minutes;
    h += mi / 60;
    mi %= 60;
    d += h / 24;
    h %= 24;
    loop {
        let len = match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            _ if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
            _ => 28,
        };
        if d <= len {
            break;
        }
        d -= len;
        mo += 1;
        if mo > 12 {
            mo = 1;
            y += 1;
        }
    }
    Some(format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}00"))
}

/// A one hour calendar file for an interview, with a short list of things to do before it.
pub fn calendar_file(
    application_id: i64,
    title: &str,
    company: Option<&str>,
    at: &str,
    posting_url: Option<&str>,
) -> Option<String> {
    let start = compact(at, 0)?;
    let end = compact(at, 60)?;
    let summary = match company {
        Some(c) => format!("Interview: {title} at {c}"),
        None => format!("Interview: {title}"),
    };
    let mut notes = String::from(
        "Before you go: reread the job posting, reread the resume you sent, and write down two \
         questions to ask.",
    );
    if let Some(u) = posting_url.filter(|u| u.starts_with("http")) {
        notes.push_str(&format!("\nPosting: {u}"));
    }
    Some(format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//CeeVee//Interviews//EN\r\nBEGIN:VEVENT\r\n\
         UID:ceevee-interview-{application_id}@ceevee.local\r\nDTSTAMP:{start}\r\n\
         DTSTART:{start}\r\nDTEND:{end}\r\nSUMMARY:{}\r\nDESCRIPTION:{}\r\n\
         BEGIN:VALARM\r\nACTION:DISPLAY\r\nDESCRIPTION:Interview in one hour\r\n\
         TRIGGER:-PT1H\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        escape(&summary),
        escape(&notes)
    ))
}

#[cfg(test)]
mod tests;
