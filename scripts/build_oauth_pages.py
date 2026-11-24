#!/usr/bin/env python3
"""Regenerate the self-contained Google sign-in result pages in crates/ceevee-google/src/pages/.

Inlines the turtle mark and the two fonts as data: URIs so the pages make no network requests.
Usage: python3 scripts/build_oauth_pages.py <mark-png-192px>
(the mark is apps/desktop/src-tauri/icons-src/ceevee-icon.png trimmed to its tile and resized to 192px)
"""
import base64, pathlib, sys

root = pathlib.Path(__file__).resolve().parent.parent
fonts = root / "apps/desktop/ui/static/fonts"
b64 = lambda p: base64.b64encode(pathlib.Path(p).read_bytes()).decode()
mark = b64(sys.argv[1])
display = b64(fonts / "CormorantGaramond-Light.woff2")
text = b64(fonts / "Inter-Regular.woff2")
label = b64(fonts / "Inter-Medium.woff2")

CSS = """
@font-face{font-family:'Cormorant Garamond';font-weight:300;src:url(data:font/woff2;base64,%s) format('woff2')}
@font-face{font-family:Inter;font-weight:400;src:url(data:font/woff2;base64,%s) format('woff2')}
@font-face{font-family:Inter;font-weight:500;src:url(data:font/woff2;base64,%s) format('woff2')}
:root{--paper:#f5f0e7;--surface:#f0e8d8;--line:#d9d2c5;--ink:#1c1c1c;--body:#3a3530;--accent:#d4a853;--danger:#9b2c2c;color-scheme:light}
@media (prefers-color-scheme:dark){:root{--paper:#1c1c1c;--surface:#282828;--line:#2e2e2e;--ink:#f5f0e7;--body:#c8b99a;--danger:#e08a8a;color-scheme:dark}}
*{box-sizing:border-box;margin:0}
body{background:var(--paper);color:var(--body);font:400 16px/1.6 Inter,system-ui,sans-serif;min-height:100vh;display:flex;flex-direction:column}
header{padding:24px 32px;border-bottom:1px solid var(--line);display:flex;align-items:center;gap:16px}
header img{width:32px;height:32px;display:block}
.word{font:300 24px/1 'Cormorant Garamond',Georgia,serif;color:var(--ink);letter-spacing:.02em}
main{flex:1;display:flex;align-items:center;justify-content:center;padding:48px 24px}
.plate{background:var(--surface);border:1px solid var(--line);width:100%%;max-width:520px;padding:48px}
.label{font:500 11px Inter,system-ui,sans-serif;letter-spacing:.3em;text-transform:uppercase;color:var(--ink);display:flex;align-items:center;gap:12px;margin-bottom:24px}
.dot{width:10px;height:10px;border-radius:50%%;background:var(--accent);display:inline-block}
.dot.err{background:var(--danger)}
h1{font:300 40px/1.15 'Cormorant Garamond',Georgia,serif;color:var(--ink);margin-bottom:16px}
h1::after{content:"";display:block;width:48px;height:1px;background:var(--accent);margin-top:24px}
p{max-width:34em;margin-top:24px}
.fine{font-size:14px}
footer{padding:24px 32px;border-top:1px solid var(--line);font-size:13px}
@media (max-width:480px){.plate{padding:32px 24px}h1{font-size:32px}}
""" % (display, text, label)

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>CeeVee · %(title)s</title>
<style>%(css)s</style></head>
<body>
<header><img alt="" src="data:image/png;base64,%(mark)s"><span class="word">CeeVee</span></header>
<main><section class="plate">
<div class="label"><span class="dot%(dotcls)s"></span>%(status)s</div>
<h1>%(h1)s</h1>
%(body)s
</section></main>
<footer>This page is served by CeeVee on your computer. It loads nothing from the internet.</footer>
</body></html>
"""

pages = {
  "oauth_done.html": dict(title="Connected", status="Connected", dotcls="", h1="Google is connected",
    body="<p>CeeVee can now read recruiter replies in your Gmail. It only reads. It never sends or deletes mail.</p>\n<p class=\"fine\">You can close this tab and go back to CeeVee.</p>"),
  "oauth_denied.html": dict(title="Access not granted", status="Not connected", dotcls=" err", h1="Access was not granted",
    body="<p>You chose not to allow access, so CeeVee did not connect to Google. Nothing was changed.</p>\n<p class=\"fine\">Close this tab and go back to CeeVee. Choose Sign in with Google to try again whenever you like.</p>"),
  "oauth_error.html": dict(title="Sign-in problem", status="Not connected", dotcls=" err", h1="Google sign-in did not finish",
    body="<p>Google returned an error before CeeVee could connect. Nothing was changed.</p>\n<p class=\"fine\">Close this tab and go back to CeeVee. Choose Sign in with Google to try again.</p>"),
}
out = root / "crates/ceevee-google/src/pages"
out.mkdir(exist_ok=True)
for name, d in pages.items():
    (out / name).write_text(PAGE % dict(d, css=CSS, mark=mark))
    print(name, (out / name).stat().st_size)
