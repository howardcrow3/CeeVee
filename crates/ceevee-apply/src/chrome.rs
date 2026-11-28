//! The real browser: Chrome or Edge driven over the DevTools protocol, behind [`BrowserDriver`].
//!
//! Fields are found by a small script that labels every visible form control with a
//! `data-ceevee-id` and reports its label, type and options. Filling sets the value the way a
//! person typing would (so the page's own scripts see `input` and `change` events). File uploads
//! go through the DevTools file-input call, which does not need an open file dialog.

use std::path::{Path, PathBuf};

use ceevee_core::{Error, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::Page;
use futures::StreamExt;
use serde_json::Value;
use tokio::task::JoinHandle;

use crate::engine::{BrowserDriver, FormField, Input};

fn err(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("browser: {e}"))
}

/// Where Chrome or Edge usually lives on the two supported systems.
pub fn find_browser() -> Option<PathBuf> {
    let candidates: &[&str] = &[
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    ];
    candidates.iter().map(PathBuf::from).find(|p| p.exists())
}

/// Script that labels the form controls and describes them. Returns a JSON string.
const LIST_FIELDS: &str = r#"(() => {
  const out = [];
  let n = 0;
  const text = (el) => (el && el.textContent ? el.textContent.replace(/\s+/g, ' ').trim() : '');
  const labelOf = (el) => {
    if (el.getAttribute('aria-label')) return el.getAttribute('aria-label').trim();
    if (el.id) {
      const l = document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
      if (l) return text(l);
    }
    const wrap = el.closest('label');
    if (wrap) return text(wrap);
    const by = el.getAttribute('aria-labelledby');
    if (by) return by.split(/\s+/).map((i) => text(document.getElementById(i))).join(' ').trim();
    return (el.getAttribute('placeholder') || el.getAttribute('name') || '').trim();
  };
  for (const el of document.querySelectorAll('input, textarea, select')) {
    const t = (el.getAttribute('type') || '').toLowerCase();
    if (['hidden', 'submit', 'button', 'image', 'reset', 'radio'].includes(t)) continue;
    const r = el.getBoundingClientRect();
    const hidden = (r.width === 0 && r.height === 0) && t !== 'file';
    if (hidden || el.disabled) continue;
    const id = 'c' + (++n);
    el.setAttribute('data-ceevee-id', id);
    let input = 'text';
    let options = [];
    if (el.tagName === 'TEXTAREA') input = 'textarea';
    else if (el.tagName === 'SELECT') { input = 'select'; options = Array.from(el.options).map((o) => o.textContent.trim()); }
    else if (t === 'checkbox') input = 'checkbox';
    else if (t === 'file') input = 'file';
    out.push({ id, label: labelOf(el), input, options, required: el.required || el.getAttribute('aria-required') === 'true' });
  }
  return JSON.stringify(out);
})()"#;

/// A running browser with one page.
pub struct ChromeDriver {
    browser: Browser,
    page: Page,
    pump: JoinHandle<()>,
}

impl ChromeDriver {
    /// Start the browser. `profile_dir` keeps the person's logins to job sites between runs.
    pub async fn launch(executable: &Path, profile_dir: &Path, headless: bool) -> Result<Self> {
        let mut cfg = BrowserConfig::builder()
            .chrome_executable(executable)
            .user_data_dir(profile_dir);
        if !headless {
            cfg = cfg.with_head();
        }
        // Linux CI containers run as root, where Chrome refuses to start without this.
        if cfg!(target_os = "linux") {
            cfg = cfg.no_sandbox();
        }
        let cfg = cfg.build().map_err(err)?;
        let (browser, mut handler) = Browser::launch(cfg).await.map_err(err)?;
        let pump = tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if event.is_err() {
                    break;
                }
            }
        });
        let page = browser.new_page("about:blank").await.map_err(err)?;
        Ok(Self {
            browser,
            page,
            pump,
        })
    }

    /// Screenshot of the page as PNG bytes, for the confirmation record.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        self.page
            .screenshot(chromiumoxide::page::ScreenshotParams::builder().build())
            .await
            .map_err(err)
    }

    pub async fn close(mut self) -> Result<()> {
        self.browser.close().await.map_err(err)?;
        self.pump.abort();
        Ok(())
    }

    /// Run a script and return its string result. For tests and diagnostics.
    pub async fn evaluate_for_test(&self, script: &str) -> Result<String> {
        let v = self.eval(script.to_string()).await?;
        Ok(v.as_str().unwrap_or("").to_string())
    }

    async fn eval(&self, script: String) -> Result<Value> {
        let r = self.page.evaluate(script).await.map_err(err)?;
        Ok(r.value().cloned().unwrap_or(Value::Null))
    }
}

fn js_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

impl BrowserDriver for ChromeDriver {
    async fn goto(&mut self, url: &str) -> Result<()> {
        self.page.goto(url).await.map_err(err)?;
        self.page.wait_for_navigation().await.map_err(err)?;
        Ok(())
    }

    async fn current_url(&mut self) -> Result<String> {
        Ok(self
            .page
            .url()
            .await
            .map_err(err)?
            .unwrap_or_else(|| "about:blank".into()))
    }

    async fn page_text(&mut self) -> Result<String> {
        let v = self
            .eval("document.body ? document.body.innerText : ''".into())
            .await?;
        Ok(v.as_str().unwrap_or("").to_string())
    }

    async fn fields(&mut self) -> Result<Vec<FormField>> {
        let v = self.eval(LIST_FIELDS.into()).await?;
        let list: Vec<Value> = serde_json::from_str(v.as_str().unwrap_or("[]")).map_err(err)?;
        Ok(list
            .into_iter()
            .map(|f| {
                let input = match f["input"].as_str().unwrap_or("text") {
                    "textarea" => Input::Textarea,
                    "checkbox" => Input::Checkbox,
                    "file" => Input::File,
                    "select" => Input::Select(
                        f["options"]
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .filter_map(|o| o.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    ),
                    _ => Input::Text,
                };
                FormField {
                    id: f["id"].as_str().unwrap_or("").to_string(),
                    label: f["label"].as_str().unwrap_or("").to_string(),
                    input,
                    required: f["required"].as_bool().unwrap_or(false),
                }
            })
            .collect())
    }

    async fn fill(&mut self, field_id: &str, value: &str) -> Result<()> {
        let script = format!(
            r#"(() => {{
              const el = document.querySelector('[data-ceevee-id=' + {id} + ']');
              if (!el) return 'missing';
              const v = {value};
              if (el.tagName === 'SELECT') {{
                const o = Array.from(el.options).find((o) => o.textContent.trim() === v);
                if (!o) return 'no such choice';
                el.value = o.value;
              }} else if (el.type === 'checkbox') {{
                el.checked = v === 'true' || v === 'yes';
              }} else {{
                const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
                Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, v);
              }}
              el.dispatchEvent(new Event('input', {{ bubbles: true }}));
              el.dispatchEvent(new Event('change', {{ bubbles: true }}));
              return 'ok';
            }})()"#,
            id = js_str(&format!("\"{field_id}\"")),
            value = js_str(value),
        );
        let r = self.eval(script).await?;
        match r.as_str() {
            Some("ok") => Ok(()),
            other => Err(err(format!(
                "could not fill {field_id}: {}",
                other.unwrap_or("no answer")
            ))),
        }
    }

    async fn upload(&mut self, field_id: &str, path: &str) -> Result<()> {
        let el = self
            .page
            .find_element(format!("[data-ceevee-id=\"{field_id}\"]"))
            .await
            .map_err(err)?;
        let params = SetFileInputFilesParams::builder()
            .files(vec![path.to_string()])
            .backend_node_id(el.backend_node_id)
            .build()
            .map_err(err)?;
        self.page.execute(params).await.map_err(err)?;
        Ok(())
    }
}
