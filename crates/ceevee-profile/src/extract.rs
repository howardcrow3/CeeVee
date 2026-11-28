//! Reading the text out of a resume file: plain text, Markdown, PDF and Word (.docx).
//!
//! Everything stays on this computer. The text is then read by the local model like pasted text.

use std::io::Read;

/// The largest file we will open. A resume is a few hundred kilobytes at most.
pub const MAX_BYTES: usize = 15 * 1024 * 1024;
/// The most text kept from one file, so a huge file cannot flood the model.
const MAX_CHARS: usize = 60_000;

/// Text from a resume file, chosen by its file name. Returns a plain sentence on failure.
pub fn text_from_file(name: &str, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_BYTES {
        return Err("That file is too large for a resume. Choose a smaller file.".into());
    }
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let text = match ext.as_str() {
        "pdf" => from_pdf(bytes)?,
        "docx" => from_docx(bytes)?,
        "doc" => {
            return Err(
                "Old .doc files are not supported. Save it as .docx or PDF and try again.".into(),
            )
        }
        "txt" | "md" | "markdown" | "" => String::from_utf8_lossy(bytes).into_owned(),
        _ => return Err("CeeVee reads PDF, Word (.docx), text and Markdown resumes.".into()),
    };
    let cleaned = tidy(&text);
    if cleaned.trim().is_empty() {
        return Err(
            "CeeVee found no text in that file. If it is a scan or an image, paste the text of your resume instead."
                .into(),
        );
    }
    Ok(cleaned)
}

fn from_pdf(bytes: &[u8]) -> Result<String, String> {
    // A damaged or unusual PDF can make the reader panic; report it as a failure instead.
    let owned = bytes.to_vec();
    std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&owned))
        .map_err(|_| "CeeVee could not read that PDF.".to_string())?
        .map_err(|_| {
            "CeeVee could not read that PDF. If it is protected or a scan, paste the text instead."
                .to_string()
        })
}

fn from_docx(bytes: &[u8]) -> Result<String, String> {
    let bad = || "CeeVee could not read that Word file.".to_string();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| bad())?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|_| bad())?
        .take(MAX_BYTES as u64 * 4)
        .read_to_string(&mut xml)
        .map_err(|_| bad())?;
    let doc = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|_| bad())?;
    let mut out = String::new();
    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        match node.tag_name().name() {
            "t" => out.push_str(node.text().unwrap_or("")),
            "tab" => out.push('\t'),
            "br" | "cr" => out.push('\n'),
            "p" if !out.is_empty() && !out.ends_with('\n') => out.push('\n'),
            _ => {}
        }
    }
    Ok(out)
}

/// Trim trailing spaces, collapse runs of blank lines and cap the length.
fn tidy(text: &str) -> String {
    let mut out = String::new();
    let mut blank = 0;
    for line in text.replace('\r', "").lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().chars().take(MAX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn docx(body: &str) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut buf);
            z.start_file(
                "word/document.xml",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            z.write_all(
                format!(
                    r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
                )
                .as_bytes(),
            )
            .unwrap();
            z.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn reads_plain_text_and_markdown() {
        assert_eq!(
            text_from_file("cv.txt", b"Ada Lovelace\r\n\r\n\r\n\r\nEngineer  ").unwrap(),
            "Ada Lovelace\n\nEngineer"
        );
        assert!(text_from_file("cv.md", "# Ada".as_bytes())
            .unwrap()
            .contains("Ada"));
    }

    #[test]
    fn reads_paragraphs_from_a_docx() {
        let bytes = docx("<w:p><w:r><w:t>Ada Lovelace</w:t></w:r></w:p><w:p><w:r><w:t>Engineer</w:t></w:r><w:r><w:tab/><w:t>2020</w:t></w:r></w:p>");
        assert_eq!(
            text_from_file("Ada.DOCX", &bytes).unwrap(),
            "Ada Lovelace\nEngineer\t2020"
        );
    }

    #[test]
    fn reads_text_from_a_pdf() {
        let t =
            text_from_file("resume.pdf", include_bytes!("../tests/fixtures/resume.pdf")).unwrap();
        assert!(t.contains("Ada Lovelace"), "{t}");
        assert!(t.contains("Software Engineer"), "{t}");
    }

    #[test]
    fn says_plainly_when_a_file_cannot_be_read() {
        assert!(text_from_file("cv.pdf", b"not a pdf")
            .unwrap_err()
            .contains("could not read that PDF"));
        assert!(text_from_file("cv.docx", b"not a zip")
            .unwrap_err()
            .contains("Word file"));
        assert!(text_from_file("cv.doc", b"x")
            .unwrap_err()
            .contains(".docx"));
        assert!(text_from_file("cv.png", b"x")
            .unwrap_err()
            .contains("PDF, Word"));
        assert!(text_from_file("cv.txt", b"   \n ")
            .unwrap_err()
            .contains("no text"));
        assert!(text_from_file("cv.txt", &vec![b'a'; MAX_BYTES + 1])
            .unwrap_err()
            .contains("too large"));
    }

    #[test]
    fn caps_very_long_text() {
        let big = "word ".repeat(40_000);
        assert_eq!(
            text_from_file("cv.txt", big.as_bytes())
                .unwrap()
                .chars()
                .count(),
            MAX_CHARS
        );
    }
}
