//! Resume and cover letter rendering to PDF with Typst, embedded as a library. The Typst source
//! is generated from the document data with every user string escaped, and uses only the fonts
//! bundled with the app, so output is identical on every machine.

use ceevee_core::{Error, Result};
use serde_json::Value;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

/// Paper size for the rendered page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Paper {
    #[default]
    UsLetter,
    A4,
}

impl Paper {
    fn name(self) -> &'static str {
        match self {
            Self::UsLetter => "us-letter",
            Self::A4 => "a4",
        }
    }
}

struct OneFileWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    main: FileId,
    source: Source,
}

impl OneFileWorld {
    fn new(text: String) -> Self {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        let path = VirtualPath::new("main.typ").expect("valid path");
        let main = FileId::unique(RootedPath::new(VirtualRoot::Project, path));
        Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main,
            source: Source::new(main, text),
        }
    }
}

impl World for OneFileWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }
    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(FileError::AccessDenied)
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

/// Escape text so Typst treats it as plain text, not markup.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(
            c,
            '\\' | '#'
                | '$'
                | '*'
                | '_'
                | '`'
                | '<'
                | '>'
                | '@'
                | '['
                | ']'
                | '~'
                | '='
                | '-'
                | '+'
                | '/'
                | '"'
                | '\''
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("").trim()
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn preamble(paper: Paper) -> String {
    format!(
        "#set page(paper: \"{}\", margin: (x: 0.8in, y: 0.7in))\n\
         #set text(font: \"Libertinus Serif\", size: 10.5pt)\n\
         #set par(justify: false, leading: 0.55em)\n\
         #show heading: set text(size: 11pt)\n\
         #show heading.where(level: 1): it => block(above: 1em, below: 0.5em)[#smallcaps(it.body) #line(length: 100%, stroke: 0.5pt)]\n",
        paper.name()
    )
}

/// Typst source for a resume in the JSON-Resume-style shape.
pub fn resume_source(resume: &Value, paper: Paper) -> String {
    let mut s = preamble(paper);
    let basics = resume.get("basics").unwrap_or(&Value::Null);
    s += &format!(
        "#align(center)[#text(size: 20pt, weight: \"bold\")[{}]\\\n",
        escape(text(basics, "name"))
    );
    let contact: Vec<String> = ["email", "phone"]
        .iter()
        .map(|k| text(basics, k))
        .chain(std::iter::once(text(
            basics.get("location").unwrap_or(&Value::Null),
            "city",
        )))
        .filter(|t| !t.is_empty())
        .map(escape)
        .collect();
    s += &format!("{}]\n", contact.join(" | "));
    let summary = text(basics, "summary");
    if !summary.is_empty() {
        s += &format!("\n= Summary\n{}\n", escape(summary));
    }
    let work = list(resume, "work");
    if !work.is_empty() {
        s += "\n= Experience\n";
        for job in work {
            s += &format!(
                "#grid(columns: (1fr, auto), [*{}*, {}], [{} to {}])\n",
                escape(text(job, "position")),
                escape(text(job, "name")),
                escape(text(job, "startDate")),
                escape(if text(job, "endDate").is_empty() {
                    "Present"
                } else {
                    text(job, "endDate")
                }),
            );
            for h in list(job, "highlights").iter().filter_map(Value::as_str) {
                s += &format!("- {}\n", escape(h));
            }
            s += "\n";
        }
    }
    let edu = list(resume, "education");
    if !edu.is_empty() {
        s += "\n= Education\n";
        for e in edu {
            s += &format!(
                "#grid(columns: (1fr, auto), [*{}*, {} {}], [{}])\n",
                escape(text(e, "institution")),
                escape(text(e, "studyType")),
                escape(text(e, "area")),
                escape(text(e, "endDate")),
            );
        }
    }
    let skills = list(resume, "skills");
    if !skills.is_empty() {
        s += "\n= Skills\n";
        for g in skills {
            let kws: Vec<String> = list(g, "keywords")
                .iter()
                .filter_map(Value::as_str)
                .map(escape)
                .collect();
            s += &format!("*{}:* {}\\\n", escape(text(g, "name")), kws.join(", "));
        }
    }
    s
}

/// Typst source for a cover letter written as plain paragraphs separated by blank lines.
pub fn letter_source(body: &str, paper: Paper) -> String {
    let mut s = preamble(paper);
    s += "#set par(spacing: 1em)\n";
    for para in body.split("\n\n") {
        let p = para.trim();
        if !p.is_empty() {
            s += &format!("{}\n\n", escape(p).replace('\n', " \\\n"));
        }
    }
    s
}

/// Compile Typst source to PDF bytes.
pub fn render_pdf(source: String) -> Result<Vec<u8>> {
    let world = OneFileWorld::new(source);
    let warned = typst::compile::<PagedDocument>(&world);
    let doc = warned.output.map_err(|errs| {
        let msg: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        Error::Other(format!("could not render the document: {}", msg.join("; ")))
    })?;
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).map_err(|errs| {
        let msg: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
        Error::Other(format!("could not write the PDF: {}", msg.join("; ")))
    })
}

pub fn resume_pdf(resume: &Value, paper: Paper) -> Result<Vec<u8>> {
    render_pdf(resume_source(resume, paper))
}

pub fn letter_pdf(body: &str, paper: Paper) -> Result<Vec<u8>> {
    render_pdf(letter_source(body, paper))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "basics": {"name": "Ada #Lovelace *", "email": "ada@example.com", "location": {"city": "London"}, "summary": "Builds $things_well."},
            "work": [{"name": "Analytical Engines", "position": "Engineer", "startDate": "2020", "highlights": ["Cut cost by 30%", "Wrote <code> & [docs]"]}],
            "education": [{"institution": "Univ", "studyType": "BSc", "area": "Math", "endDate": "2019"}],
            "skills": [{"name": "Languages", "keywords": ["Rust", "C++"]}]
        })
    }

    #[test]
    fn resume_renders_to_a_pdf() {
        let pdf = resume_pdf(&sample(), Paper::UsLetter).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 1000);
    }

    #[test]
    fn a4_and_letter_both_render() {
        assert!(resume_pdf(&sample(), Paper::A4)
            .unwrap()
            .starts_with(b"%PDF-"));
    }

    #[test]
    fn markup_characters_in_text_cannot_inject_code() {
        let r = json!({"basics": {"name": "#panic(\"x\")"}});
        assert!(resume_pdf(&r, Paper::UsLetter).is_ok());
        assert_eq!(escape("#a *b* _c_"), "\\#a \\*b\\* \\_c\\_");
    }

    #[test]
    fn empty_resume_still_renders() {
        assert!(resume_pdf(&json!({}), Paper::UsLetter).is_ok());
    }

    #[test]
    fn letter_renders() {
        let pdf = letter_pdf(
            "Dear team,\n\nI am keen to apply.\n\nThanks,\nAda",
            Paper::UsLetter,
        )
        .unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }
}
