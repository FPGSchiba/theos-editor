# Tauri 2 webviews — dossier

Researched: Tauri 2.12.5 (crates.io, 2026-10-21), wry 0.57.1 (2026-10-23), tao 0.37.2 (2026-10-23), tauri-runtime-wry 2.12.3 (2026-10-21), Playwright 1.64.1 (npm, 2026-10-14) (read 2026-10-31) · Researcher: P10 Task 3

Scope: what the three webviews of G2 (WebView2, WKWebView, WebKitGTK) do for native spell check, IME composition and dead keys, as hosted by Tauri 2 / wry, and how Playwright's engines relate. Facts only; the source of every fact is in the tag. "Code reading" means I read the source and did not run it.

## Spell check

Cross-engine and Tauri-level facts

- HTML Standard: the `spellcheck` attribute has states True (`""` or `"true"`) and False (`"false"`); a missing or invalid value is the Default state, whose behaviour is chosen by the user agent (true-by-default, false-by-default or inherit-by-default per element). Checkable text includes text nodes of editing hosts such as `contenteditable`. The `spellcheck` IDL attribute "is not affected by user preferences that override the spellcheck content attribute" [D62].
- The string "spell" does not occur (case-insensitive search of the crate tarballs) in any file of Tauri 2.12.5 or tauri-runtime-wry 2.12.3, nor in the Rust, Markdown and TOML files of wry 0.57.1. Tauri and wry therefore have no spell-check switch, no spelling-language setting and no spelling menu item [D1, D3, D4].
- Escape hatches that exist: `WebviewWindow::with_webview` hands over a `PlatformWebview`: on Linux `inner()` returns a `webkit2gtk::WebView`; on Windows `controller()` and `environment()` return the WebView2 objects; on macOS `inner()` returns the raw `WKWebView` pointer [D3].
- Default native context menus are left alone on all three platforms: wry only sets `AreDefaultContextMenusEnabled` for WebView2, from `default_context_menus` whose default is `true` ("This is WebView2's default behavior"); Tauri 2.12.5 and tauri-runtime-wry 2.12.3 never call `with_default_context_menus`. wry has no context-menu handler for WKWebView or WebKitGTK [D1, D3, D4].
- Tauri's default application menu (2.12.5) has an Edit submenu of undo, redo, cut, copy, paste and select all. The predefined menu items of muda 0.20.0 (the version Tauri 2.12.5 depends on) have no spelling item, so there is no "Spelling and Grammar" entry unless the app builds one [D3, D5].
- On Windows, Tauri's window config has `additionalBrowserArgs` and the builder has `additional_browser_args` and `with_environment` (a custom `ICoreWebView2Environment`) [D3, D6]. wry's default args are `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`; passing your own args replaces them, and webviews with different browser args need different data directories [D1].

### WebView2 (Windows)

On or off
- Spell check is on in a new WebView2. A 2020 report says "By default the WebView2 browser has spellchecking enabled" and a Microsoft engineer answered with the per-element `spellcheck` attribute, not by disputing it [D14]. Microsoft's table of differences between Edge and WebView2 has no spell-check row [D13]. A 2026 report underlines misspelled words in a bare Win32 WebView2 host with default settings [D24].
- It works in a `contenteditable` with `spellcheck="true"`: a 2026-09-23 report shows underlines in `<div contenteditable="true" spellcheck="true">` when the user focuses it with the mouse, on runtime 153.0.4234.48, in a bare Win32 host and "also reproduced through wry 0.55.1 / Tauri 2" [D24].
- Per element, `spellcheck="false"` switches it off; a Microsoft engineer proposed this in 2020 and the reporter answered it only works for content they control [D14].
- There is no WebView2 API or setting for it. The `webview2-com-sys` 0.39.1 bindings (generated from the WebView2 SDK, published 2026-03-11) contain no identifier with "spell" and list `ICoreWebView2Settings` up to `Settings9` [D10]. Requests: "Add ability to disable spell checking" (#584, open since 2020-10-28), "API for Spell Checker" (#3758, closed 2023-10-20 as duplicate of #2040), "Add option to change spellchecking method" (#4840, open) [D14, D17, D18].

Engine and network
- Before the runtime-106 change the engine was the local Windows spell checker: the NVD entry for CVE-2023-22880 says a WebView2 runtime update "transmitted text to Microsoft's online Spellcheck service instead of the local Windows Spellcheck" and that runtime 109.0.1481.0 or later fixes the behaviour [D23].
- Upstream Chromium builds on Windows with both a browser (platform) spell checker and the renderer (Hunspell) one: `use_browser_spellchecker = is_android || is_mac || is_win`, `use_renderer_spellchecker = !use_browser_spellchecker || is_win` [D34]. Edge's own build is closed source; how WebView2 combines them is not stated in the sources I read.
- Runtime 106 sent typed text to `https://nleditor.osi.office.net/NLEditor/TileCheck/V1` (Microsoft Editor); Microsoft said WebView2 "has this feature on by default", gave the flag `--disable-features=msUseSpellCheckCorrectionsCard`, and fixed it by disabling the feature in 109.0.1481.0+ [D21, D22].
- A 2024 reporter says WebView2 "seems to behave like it's using the basic method" (Edge offers "Basic" and "Microsoft Engine"); the same reporter asks for the Edge policies `SpellcheckEnabled` and `MicrosoftEditorProofingEnabled` under `SOFTWARE\Policies\Microsoft\Edge\WebView2`; the issue has no reply [D18].
- Upstream Chromium downloads Hunspell dictionaries from `https://redirector.gvt1.com/edgedl/chrome/dict/` (`kDownloadServerUrl`). Whether WebView2 downloads dictionaries is not stated in the sources I read [D34].

Languages
- wry creates the WebView2 environment with `Language` set to the locale name of the Windows UI language (`GetUserDefaultUILanguage` then `LCIDToLocaleName`) [D1].
- A Microsoft engineer (2021, 2022) confirms the environment `Language` option affects the spell-check language, and the reporter saw the matching dictionaries become available without installing anything [D15, D16]. In 2021 a reporter saw only English (US, UK) loaded by default [D15].
- The HTML `lang` attribute is ignored by the spell checker: the spell-check language follows only the environment `Language` (#5294, open, reproduced by Microsoft 2025-09-16, runtime 137) [D19].
- A language tag with a region can be missing from the supported list: "da-DK" was "not in the support list", `Language = "da"` worked (2024-03-21); Microsoft gave no date for a fix [D20]. Dutch suggestions were empty for `nl-NL` in 2023 (#3237, open) [D27].
- For a WinUI app a Microsoft engineer suggested `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--lang=<tag>` as the way to set the spelling language without the environment option, and said he had not checked which wins [D16]. How that interacts with wry's `set_language` is not tested.

Context menu and suggestions
- Suggestions are in the default WebView2 context menu; the entry first reads "Getting suggestion" and fills in asynchronously. With a `ContextMenuRequested` handler that takes more than a few milliseconds the suggestions never appeared (#3142, closed 2023-03-06, tracked in #2340) [D26]. With a custom menu built from `ContextMenuRequested`, the menu items named `spellcheck` carry no suggestion text (#2340, open, last activity 2025-10-31) [D25]. wry installs no `ContextMenuRequested` handler [D1].
- Other open reports: a correction chosen from the menu sometimes does not change the text (#3527, created 2023-05-31); choosing from a left-click spelling menu moves focus out of the WebView, with CKEditor 4 replacing the right-click menu (#3012, created 2022-12-01; Microsoft said that version of spell check "has been disabled in WebView2 and is reverted to the old spellcheck feature") [D27].
- A "corrections card" opened on a plain click of a misspelled word in runtime 106.0.1370.42; disabled in 109.0.1481.0+ [D22].

Focus behaviour (affects ProseMirror-style editors that focus on start)
- #5728 (open as of 2026-10-31, created 2026-09-23, last activity 2026-10-08, no comments): a `contenteditable` focused by script (`focus()` or `getSelection().collapse`) gets no underline for later typing; a `<textarea>` underlines; clicking out and back in with the mouse fixes it and the earlier words get underlined; toggling `spellcheck`, `blur()` then `focus()`, and clicking into the already focused element do not help; replacing the element does. The report names "ProseMirror / Tiptap based apps" [D24].

Chromium-level reports outside WebView2 (not the same embedder)
- A CodeMirror 6 editing host got no native marks in Electron 44 (Chromium 152) while a plain `contenteditable` did; the same editor was checked in Chrome 152.0.7977.82 on Linux; a second user reported no marks in Chrome 152 (2026-09-09), unanswered [D69].

### WKWebView (macOS)

On or off
- Continuous spell checking is off unless the app's defaults say otherwise. WebKit builds its text-checker state once, at first use, from `[[NSUserDefaults standardUserDefaults] boolForKey:@"WebContinuousSpellCheckingEnabled"]` (absent key reads as NO) and `isContinuousSpellCheckingAllowed()` (the `NSAllowContinuousSpellChecking` default can disallow it). The menu action later writes the key back [D36].
- Tauri #7705 (open as of 2026-10-31, created 2023-08-28, label "status: needs triage"): a Tauri app showed no red underlines in an input on macOS 13.5 (also under `tauri dev`) while right-click still offered the native spelling options; turning on "Check spelling while typing" in the app fixed it. A commenter (pewsheen, 2023-09-04) found the `WebContinuousSpellCheckingEnabled` key: `defaults write <bundle id> WebContinuousSpellCheckingEnabled -bool true`, or `setBool:forKey:` on `NSUserDefaults`; "if we decide to set it by the code, then we need to call it before the webview is created otherwise it requires restarting the app to take effect". A 2025-07-13 comment: underlines worked under `tauri dev` but not in the built app, and setting the key in `setup` was too late (it took effect on the next run). Elements other than `<input>` need `contenteditable` [D37].
- Tauri's default menu has no "Check Spelling While Typing" item (see the Tauri-level facts) [D3, D5].
- WKWebView honours the `spellcheck` attribute as an element-level gate on top of that state: WebKit's `Editor::isSpellCheckingEnabledFor` ends in `element->isSpellCheckingEnabled()` and misspelling marking requires `isContinuousSpellCheckingEnabled()` and an editable node [D40].

Engine and languages
- The engine is the OS: `NSSpellChecker` (`checkString:range:types:…inSpellDocumentWithTag:`). The language is detected per text: guesses use `languageForWordRange:inString:orthography:` from an orthography check of the surrounding text. wry has no per-webview language setting [D36, D1].

Suggestions
- Right-click suggestions worked in the default menu even while the underlines were hidden: "if you right click on a misspelled word the native spellchecker options still appear" (macOS 13.5, Tauri, 2023) [D37]. wry has no context-menu handler for WKWebView [D1].

Smart substitutions (touches byte-stability, criterion 1)
- WebKit's initial state for automatic quote substitution, dash substitution, text replacement and spelling correction follows the macOS system settings (`NSSpellChecker isAutomaticQuoteSubstitutionEnabled` and the like) unless the app's defaults hold `WebAutomaticQuoteSubstitutionEnabled`, `WebAutomaticDashSubstitutionEnabled`, `WebAutomaticTextReplacementEnabled` or `WebAutomaticSpellingCorrectionEnabled` [D36].
- In `Editor::markMisspellingsAfterTypingToWord` the macOS path requests a Replacement check whenever quote substitution, link detection, dash substitution or text replacement is on, independent of continuous spell checking (code reading) [D40].
- Tauri #12286 (closed 2025-01-07, Tauri 2.2.0, macOS 15.0.1): with an English input source, typed `"` and `'` became Chinese quotation marks; a commenter's workaround was `spellcheck="false"` on `<html>` [D38]. Whether this happens in a `contenteditable` with `spellcheck="true"` under default system settings is not established by the sources.

Safari behaviour reported by editor maintainers
- CodeMirror's maintainer, about Safari on iOS (2022-03-14): Safari does not spell-check "when the editable DOM is changed programmatically"; ProseMirror reduces the problem by avoiding unnecessary DOM changes. The poster saw marks on macOS Chrome and Safari with `spellcheck="true"` [D70].

### WebKitGTK (Linux)

On or off
- Off by default. `checkerState()` in `TextCheckerGLib.cpp` is a static empty option set, so `ContinuousSpellCheckingEnabled` is not set; `TextCheckerEnchant::hasDictionary()` is false until `updateSpellCheckingLanguages` has run. A GitHub code search of WebKit/WebKit for `setSpellCheckingLanguages` returned only the GLib and C API wrappers, the header, the implementation and the GTK and WPE test runners, so the library itself does not call it at start-up [D47, D48].
- The API has two calls and both are needed. `webkit_web_context_set_spell_checking_enabled(context, enabled)` ("Enable or disable the spell checking feature") and `webkit_web_context_set_spell_checking_languages(context, languages)`: "You need to call this function with a valid list of languages at least once in order to properly enable the spell checking feature in WebKit" [D46, D49]. The state is process-wide (static `checkerState()`) [D47].
- The Rust side exists: `webkit2gtk` 2.0.1 has `WebContextExt::set_spell_checking_enabled(bool)` and `set_spell_checking_languages(&[&str])`; tauri-runtime-wry 2.12.3 depends on `webkit2gtk =2.0` with feature `v2_40` [D9, D4]. Neither wry nor Tauri calls or exposes them; an app can reach the `WebView` (and its `WebContext`) through `with_webview` [D1, D3].

Engine and languages
- The engine is Enchant (`TextCheckerEnchant`), a wrapper over Hunspell, Aspell and other providers. Language codes are of the form `lang_COUNTRY` (`sv_FI`, `pt_BR`). A non-empty list loads each dictionary that `enchant_broker_dict_exists`. An empty list falls back to `defaultLanguage()` (first user-preferred language) and then to the first dictionary Enchant lists (code reading; `g_return_if_fail(languages)` rejects only a NULL list) [D46, D48].
- A word is accepted if any loaded dictionary accepts it; suggestions are up to 10 per dictionary, concatenated [D48]. The `lang` attribute is not consulted by `TextCheckerEnchant` (code reading) [D48].
- The Debian trixie package `libwebkit2gtk-4.1-0` lists `libenchant-2-2` ("Wrapper library for various spell checker engines") among its runtime dependencies [D55]. Which Enchant providers and dictionaries a user's system has is not stated by WebKitGTK, wry or Tauri.

Suggestions
- WebKitGTK's own context menu has the actions `SPELLING_GUESS`, `NO_GUESSES_FOUND`, `IGNORE_SPELLING` and `LEARN_SPELLING` [D52]. wry connects no context-menu handler [D1].

## IME and dead keys

Cross-engine facts

- UI Events (Editor's Draft, 21 February 2026), 4.3.2: dead keys have the key value `Dead`; "The MacOS and Linux operating systems use input methods to process dead keys"; "In response to any dead key press, composition events must be dispatched by the user agent". The example sequence is keydown `Dead` (isComposing false), compositionstart, compositionupdate U+0302, keyup, keydown `ê`, compositionupdate `ê`, compositionend `ê` [D63].
- In January 2016 an Apple engineer wrote that Chrome, Firefox and Safari "fire composition events for dead keys on Mac but they don't on Windows"; on Windows the dead key inserts nothing and the second key inserts the composed character; on Mac the dead key inserts the character and fires `compositionstart`, the second key replaces it and fires `compositionend` [D64]. In the same thread: on Linux the first key inserts nothing and fires no composition events; Firefox Nightly fires them after the second key; "Chrome(49) doesn't seem to dispatch any composition events in this case on linux" [D65]. These are 2016 statements, not re-measured here.
- MDN: `compositionstart` may fire after `keydown` for the first character, and `compositionend` may fire before `keydown` for the last one; `isComposing` is then false but `keyCode` is still 229 [D66].
- Editor maintainers' source notes: CodeMirror ignores the Enter `keydown` that follows `compositionend` on Safari because "compositionend and keydown events are sometimes emitted in the wrong order", and its changelog for view 6.26.1 (2024-03-28) says "Fix the editor getting stuck in composition when Safari fails to fire a compositionend event for a dead key composition" [D67]. ProseMirror ignores one Safari `keydown` within 500 ms of `compositionend` [D68]. ProseMirror view changelog: 1.41.0 (2025-09-06) "a Safari issue that broke composition right after changing the set of active marks"; 1.41.4 (2025-12-01) "a composition problem on Chrome Windows when starting a composition before an absolutely positioned widget"; 1.41.5 (2026-01-14) "a newly introduced bug in Safari where composition in an empty table cell causes it to move the text out of the cell"; 1.34.2 (2024-09-03) "for some kinds of IME on Windows, pressing Enter while in a composition would execute the resulting command on the wrong selection" [D68].

### WebView2 (Windows)

Composition
- The 2026 WebView2 reports test IME through the standard events: the #5625 reproduction page "logs IME events" and the reporter offers traces of "keydown / compositionstart / compositionend / beforeinput / input" from a CodeMirror 6 editor on WebView2 154.0.4258.53 [D28].
- #5625 (open as of 2026-10-31, created 2026-06-19, last activity 2026-10-05), host Tauri 2, runtime 149.0.4022.69: in a `contentEditable` the first character of a composition is dropped and full-width punctuation needs two key presses; a `<textarea>` in the same page is correct; severe with Sogou Pinyin, mild with Microsoft Pinyin. The reporter says JS-side suppression of DOM mutations between `compositionstart` and `compositionend` does not help. An update of 2026-10-05 on 154.0.4258.53: idle runs were exact for both IMEs; while the renderer main thread was busy (app start-up), 4 of 5 Sogou runs lost the first letter after a commit (`nihao` became `ihao`) [D28].
- A separate third-party write-up says that during the first abnormal IME input neither the editor's nor external listeners received `compositionstart`, `beforeinput`, `input` or `compositionend`, in Chromium 149.0.7827.103+ on Windows (Chrome, and Edge 149.0.4022.62); Edge 149.0.4022.52 (Chromium 149.0.7827.54), Firefox and Electron 39 (Chromium 142) were not affected; the author filed Chromium issue 523134891 (the tracker page is not readable without sign-in) [D35].
- Earlier: #1610 (created 2021-08-05, closed 2024-09-05) uncommitted Japanese or Chinese text was duplicated after focus left and returned; "fixed in runtimes 125+" [D29].
- #5675 (open, created 2026-08-18): the composition window jumps to the top-left of the screen after the host window is resized; Korean IME, plain `<input>`; Edge and Chrome do not do it [D30]. wry #1908 and Tauri #16391 (both open, 2026-10-24): composition UI at screen position (0, 0) after switching between two windows of the same application; reproduced on Tauri 2.12.5 / wry 0.57.1 / tao 0.37.2; the wry report has a standalone wry + tao reproduction without Tauri; Microsoft Pinyin and WeChat IME [D56, D57].
- Tauri #15436 (open, created 2026-05-24; WebView2 148.0.3967.83, Tauri 2.11.2, wry 0.55.1): when a `<textarea>` or a ProseMirror/TipTap `contenteditable` already holds text, the first focus freezes TSF for CJK IMEs (Microsoft Bopomofo) until an English character is typed or focus moves; empty inputs are fine [D58].
- tao #1349 (closed 2026-09-28): a `PeekMessageW` re-entrancy deadlock in tao's keyboard/IME handling hung the main thread on Tauri 2.10.3 / tao 0.34.8; the reporter confirmed Tauri 2.12 (tao 0.37.1) fixed it, "keyboard input (including IME)... behave correctly" [D60].
- The reporter of #5625 argues the first-character loss is "below the DOM/JS layer in WebView2's TSF text input" [D28]. wry 0.57.1 creates the controller options without `AllowHostInputProcessing`; wry #1836 (open, created 2026-09-09) asks to expose it [D78].

Dead keys
- Tauri / tao on Windows: tao #1408 (open as of 2026-10-31, created 2026-10-30, no reply): on Windows 11 with a German layout (`^`, `´`), Tauri 2.12.5 / tao 0.37.2 / WebView2 156 can panic in the `WM_DEADCHAR | WM_SYSDEADCHAR` branch at `keyboard.rs:177` (`self.event_info.lock().take().unwrap()` on `None`) when the `WM_DEADCHAR` arrives after the key-down was dispatched alone; timing-dependent, seen while the UI thread was busy. The line and the code are in tao 0.37.2 as quoted [D59, D7].
- Whether Chromium/WebView2 on Windows dispatches composition events for dead keys today is not settled by a current primary source (2016 statement above; UI Events text says macOS and Linux use input methods) [D63, D64].

### WKWebView (macOS)

- wry adds no text-input code to WKWebView. `WryWebView` overrides `performKeyEquivalent:` (returns NO for child webviews) and `acceptsFirstMouse:`; `WryWebViewParent` overrides `keyDown:` (forwards to the main menu's `performKeyEquivalent`) and `drawRect:`. A search of `src/wkwebview` for `insertText`, `setMarkedText`, `interpretKeyEvents`, `inputContext` and `NSTextInput` found nothing, so composition and dead keys are handled by WebKit itself [D1].
- WebKit starts a composition for dead keys: WebKit bug 167880 (created 2017-02-06, status NEW as of 2026-10-31, last change 2022-07-26): on a German Mac keyboard `^`, `´` and `` ` `` start a composition in a `contenteditable`; the `keypress` after the composition carries the dead key's data, not the typed key's; "Safari 15.6 matches with Chrome" [D41]. A 2016 description of the same OS behaviour is in [D64].
- Press-and-hold accents on macOS (hold `e`, pick a variant) fire no `compositionstart` or `compositionend`; the page sees repeated key downs and then an `input` with replaced text (Playwright issue, 2021-03-10) [D77].
- Event order for the key that commits an IME (Enter): WebKit bug 165004 (created 2016-11-21; RESOLVED as duplicate of 311717) says Safari sent `compositionend` before that `keydown`, so `isComposing` was false [D42]. Bug 311717 (created 2026-04-08, RESOLVED FIXED, "turn on correct composition event ordering by default", flag `InputMethodUsesCorrectKeyEventOrder`) was reverted by bug 317127 (created 2026-06-15, RESOLVED FIXED, "Disable InputMethodUsesCorrectKeyEventOrder"; "We've been getting too many regression reports. Disabling this behavior everywhere."). Bug 324380 (NEW, 2026-09-16, Safari 26 on macOS 26) reports the legacy order in macOS Web Apps and a Messenger report on macOS 27 shows `compositionend` before `keydown` with `isComposing=false` (2026-10-16) [D43, D44, D45]. All five bug records checked through the Bugzilla REST API on 2026-10-31. Which WebKit build a given macOS version ships, and whether WKWebView in a Tauri app reads the flag, is not stated.
- Tauri #15924 (open as of 2026-10-31, created 2026-08-27, "status: needs triage"): on macOS a third-party IME (WeChat/WeType) drops the first keystroke (`Shift+;`) inside the Tauri WebView; Safari with the same IME is fine; the built-in Pinyin IME is fine. A commenter could not reproduce on macOS 26.2.0 with Tauri 2.11.5, tao 0.35.3, wry 0.55.1 and WeType 2.2.3 [D39].
- The WebKit that WKWebView uses is the one in macOS; "unsupported macOS versions do not receive WebKit updates" [D11].

### WebKitGTK (Linux)

- wry turns IME preedit off on every WebKitGTK webview: `input_context.set_enable_preedit(false)` in `set_webview_settings` ("Disable input preedit, fcitx input editor can anchor at edit cursor position") [D1]. wry 0.29.0 changelog (PR #955, fix for Tauri #5986): "With input method preedit disabled, fcitx can anchor at edit cursor position. the pre-edit text will not disappear, instead it shows in the fcitx selection window below the input area" [D2]. GTK: with `use_preedit` FALSE "the IM context may use some other method to display feedback, such as displaying it in a child of the root window" [D51]. Fixed by this change: Tauri #8264 (Tauri 1, fcitx5, closed 2026-05-06; "The fix is in Wry 0.29.0, but it hasn't been backported to Tauri v1") [D61].
- Path inside WebKitGTK (code reading): `InputMethodFilter` turns `preedit-started` and `preedit-changed` into `webkitWebViewSetComposition` and `committed` into `webkitWebViewConfirmComposition`; a one-character commit with no preedit change is treated as a plain key press ("Simple input methods work such that even normal keystrokes fire the commit signal without any preedit change") [D50]. In WebCore `Editor::confirmComposition(text)` calls `setComposition(text, ConfirmComposition)`, which inserts the text and dispatches `compositionend` with it; `compositionstart` and `compositionupdate` are dispatched from the `setComposition` path that preedit uses [D40]. Reading these together: with wry's preedit off, a multi-character commit would arrive as `compositionend` without a preceding `compositionstart` and a one-character commit as ordinary key input. This was not run (see Unknowns).
- Tauri #11412 (open as of 2026-10-31, created 2024-10-19, last activity 2026-08-26): Japanese IME window appears far from the input in Tauri v2 on Linux (fcitx/Mozc, ibus/Mozc); a 2026-08-26 reproduction (Ubuntu 26.04, GNOME 50.1, Wayland, fcitx5 5.1.19, WebKitGTK 2.52.3, Tauri 2.11.5, wry 0.55.1) narrowed it to the IME cursor rectangle after the document scrolls; `input`, `textarea` and `contenteditable` alike [D53].
- Tauri #15859 (open as of 2026-10-31, created 2026-08-10, label "status: upstream"): the Compose key (right Alt as Compose, `'` then `e`) does nothing in a Tauri app on KDE/Wayland (webkit2gtk-4.1 2.52.5, wry 0.55.1) and the console prints "Couldn't get key from code: AltRight"; the reporter says it also fails in Epiphany; GIMP and gedit compose fine; a maintainer attributes it to WebKitGTK. The reporter later wrote that IME input worked after switching UI library, "it did, and IME input even works in Tauri now" [D54].
- Dead keys under GTK input methods: UI Events says Linux uses input methods for dead keys [D63]; no Tauri, wry or WebKit source I read measures dead-key events in WebKitGTK.

## Playwright engines versus the webviews

Builds (Playwright 1.64.1, published 2026-10-14)

| Engine | Playwright 1.64.1 | Webview in Tauri 2 | Source |
|---|---|---|---|
| Chromium | "Chrome for Testing" 156.0.8078.4 (headed) and "Chrome Headless Shell" 156.0.8078.4 (default headless); release notes: "Chromium 156.0.8078.4", also tested against Chrome 155 and Edge 155 | WebView2 Evergreen runtime, updates itself; versions seen: 153.0.4234.48 (report 2026-09-23), 154.0.4258.53 (2026-10-05), 154.0.4258.31 released 2026-09-28, "Edge 156" (2026-10-30 report) | [D71, D72, D74, D24, D28, D32, D59, D11] |
| WebKit | revision 2370, "27.2" (macOS 14 uses revision 2251); built from WebKit main, "often built before those changes reach Apple Safari"; "does not work with branded Safari" | macOS: the WebKit in the OS (WKWebView), version tied to the OS. Linux: the distro's WebKitGTK 4.1 (Tauri builds against the 2.40 API; Linux reports show 2.44.3 to 2.52.5). Windows: none | [D72, D74, D11, D4, D53, D54] |
| Firefox | 157.0 | not used by Tauri | [D72] |

- Playwright's WebKit is not the system WebKit. On macOS the dev script runs `Playwright.app` with its own frameworks (`DYLD_FRAMEWORK_PATH`). On Linux headed runs use the GTK build (`WebKitBuild/GTK`) and `--headless` selects the WPE build (`WebKitBuild/WPE`); the launcher adds `--headless` when the browser runs headless. Windows has a `webkit-win64.zip` build with its own embedder (`embedder/Playwright/win`); which WebKit port it uses is not stated in the files I read. A channel `webkit-wsl` runs the Linux build inside WSL [D75, D73].
- Playwright launches Chromium with `--disable-field-trial-config`, `--disable-component-update`, `--disable-background-networking` and a `--disable-features=` list (including `Translate`, `msForceBrowserSignIn`); none of the switches concerns spell check or IME [D73]. A WebView2 launched by wry gets `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection` [D1].
- Playwright says WebKit on macOS gives the "closest-to-Safari experience"; it names no mapping from its WebKit build to a Safari or WebKit version number [D74].

Driving IME and dead keys

- Playwright has no composition API. Issue #5777 "IME support" (opened 2021-03-10) proposed `startComposition`, `updateComposition`, `endComposition`; it was closed on 2026-10-31 by a bulk closure of `P3-collecting-feedback` issues ("This doesn't mean the request is wrong"), with no API added; the 1.64.1 package contains no call to `Input.imeSetComposition` or `Page.setComposition` (search count 0) [D77, D73].
- Text insertion: `keyboard.insertText` "Dispatches only `input` event, does not emit the `keydown`, `keyup` or `keypress` events"; it maps to `Input.insertText` on Chromium and `Page.insertText` on WebKit [D73].
- Chromium: CDP `Input.imeSetComposition` (experimental; parameters `text`, `selectionStart`, `selectionEnd`, optional `replacementStart`, `replacementEnd`; "Use imeSetComposition with empty string as text to cancel composition") and `Input.insertText` ("emulates inserting text that doesn't come from a key press, for example an emoji keyboard or an IME"; experimental) [D76]. Playwright exposes CDP through `context.newCDPSession(page)`, "only supported on Chromium-based browsers" [D73]. A Playwright engineer showed this works for set and cancel; the description mentions `imeCommitComposition`, which is not a command of the Input domain; commit is done with `Input.insertText` (it ends in the same Chromium call) [D76, D77]. The Quill repository's e2e `Composition.ts` fixture uses this approach and "covers all the IME issues we've encountered" (comment of 2023-11-15) [D77].
- WebKit: no composition input. A 2023-09 workaround via the internal session sent `Page.setComposition`; on 2023-11-30 WebKit answered "'Page.setComposition' was not found" [D77]. Not re-tested on 1.64.1.
- Firefox: composition needs changes in Firefox's `juggler`; PR #9921 "incomplete" (2023-11-15) [D77].
- Dead keys: Playwright's keyboard maps the US layout ("We currently have the US keyboard layout mapped out"; dead keys and other layouts "not too hard" to add) [D77]. The P10 spec's dead-key sequences (`^` then `ê`) would be CDP `Input.imeSetComposition` followed by `Input.insertText` on Chromium; no equivalent on WebKit.
- None of these paths goes through the operating system's IME or the hosting window. For WebView2, the 2026 reports above locate failures in TSF and in the host window (caret rectangle, focus), which CDP-injected composition does not exercise [D28, D30, D56].

Driving the real webviews

- Playwright can attach to a running WebView2 over CDP: set `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>` (or the control's `AdditionalBrowserArguments`), then `chromium.connectOverCDP('http://localhost:9222')`; Windows 10 and 11 only; parallel runs need distinct `WEBVIEW2_USER_DATA_FOLDER` values [D31]. The WebView2 API also has `ICoreWebView2::CallDevToolsProtocolMethod` for CDP from the host [D33]. Whether `Input.imeSetComposition` is honoured through either path is not tested.
- No CDP for WKWebView or WebKitGTK in the sources read. Tauri's WebDriver docs: `tauri-driver` drives Windows and Linux; "macOS has no WKWebView driver tool available"; the service uses an embedded server for macOS, and CrabNebula's fork needs "a paid API key" for macOS [D12].

## Known open issues

State is as of 2026-10-31 unless a date is given. "Criterion 6" is Spell check and IME; "criterion 1" is Byte-stable Markdown.

- Spellcheck never runs in a contenteditable after programmatic focus, until the user refocuses it by mouse (WebView2Feedback #5728) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5728 · open, created 2026-09-23, last activity 2026-10-08, no replies · criterion 6: native marks in a script-focused editor on Windows [D24]
- WebView2 drops first CJK IME character and doubles Chinese punctuation in contentEditable (#5625) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5625 · open, created 2026-06-19, last activity 2026-10-05 · criterion 6: composition correctness on Windows (Chinese IMEs) [D28]
- IME composition window jumps to the top-left of the screen after the host window is resized (#5675) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5675 · open, created 2026-08-18 · criterion 6: composition UI position on Windows [D30]
- Spellchecker ignores HTML `lang` attribute, uses only Environment `Language` setting (#5294) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5294 · open, created 2025-07-02, Microsoft reproduced 2025-09-16 · criterion 6: language of spelling marks [D19]
- Spell check suggestion text is not available in ContextMenuRequested event (#2340) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/2340 · open, created 2022-04-06, last activity 2025-10-31 · criterion 6: only matters if the app replaces the context menu [D25]
- Add ability to disable spell checking (#584); Add option to change spellchecking method (#4840); Spellchecking languages other than English (#2040) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/584 · open (since 2020-10-28, 2024-09-30 and 2021-12-22) · criterion 6: no WebView2 switch or language API [D14, D18]
- Spell check correction from context menu doesn't always work (#3527); Left click spellcheck menu causes WebView to lost focus (#3012); Webview2 spellcheck suggestions empty for Dutch language (#3237) · https://github.com/MicrosoftEdge/WebView2Feedback/issues/3527 · open (since 2023-05-31, 2022-12-01 and 2023-02-24) · criterion 6: suggestion flow on Windows [D27]
- [Windows] WebView2 IME/TSF freezes on first focus of inputs with existing text (CJK input methods blocked) (Tauri #15436) · https://github.com/tauri-apps/tauri/issues/15436 · open, created 2026-05-24, status "needs triage" · criterion 6: names ProseMirror/TipTap contenteditable [D58]
- [Windows] IME composition/candidate UI loses caret positioning when switching between windows of the same application (Tauri #16391, wry #1908) · https://github.com/tauri-apps/tauri/issues/16391 · open, created 2026-10-24 · criterion 6: only for multi-window use [D57, D56]
- [Windows] Panic on a dead key: unwrap() on None in the WM_DEADCHAR branch of keyboard.rs (tao #1408) · https://github.com/tauri-apps/tao/issues/1408 · open, created 2026-10-30, no replies; the report is by one user and was not reproduced by anyone else · criterion 6: dead keys on German/Swiss layouts on Windows [D59]
- macOS: third-party IME drops the first keystroke inside the WebView (Safari with the same IME is unaffected) (Tauri #15924) · https://github.com/tauri-apps/tauri/issues/15924 · open, created 2026-08-27; one commenter could not reproduce · criterion 6: IME on macOS [D39]
- Safari MacOS Spelling is Hiding the Styles / Underline for Errors (Tauri #7705) · https://github.com/tauri-apps/tauri/issues/7705 · open, created 2023-08-28, last activity 2025-07-13, "needs triage" · criterion 6: spell check is off by default in WKWebView [D37]
- IME window position appears out of input/textarea (cannot inline-input) on Tauri v2 apps in Linux (Tauri #11412) · https://github.com/tauri-apps/tauri/issues/11412 · open, created 2024-10-19, last activity 2026-08-26 · criterion 6: IME on Linux [D53]
- Tauri doesn't respect Compose key on Linux (KDE) (Tauri #15859) · https://github.com/tauri-apps/tauri/issues/15859 · open, created 2026-08-10, "status: upstream" · criterion 6: dead keys and compose sequences on Linux [D54]
- WebKit provides a keypress event with wrong which/keyCode/charCode directly after a composition started with ^,´,` completes (WebKit 167880) · https://bugs.webkit.org/show_bug.cgi?id=167880 · NEW, created 2017-02-06, last change 2022-07-26 · criterion 6: dead keys on macOS [D41]
- "Correct key event ordering with composition events" Feature Flag does not affect Web Apps (WebKit 324380) · https://bugs.webkit.org/show_bug.cgi?id=324380 · NEW, created 2026-09-16, last change 2026-10-16 · criterion 6: Enter after IME commit on macOS; related 311717 (turn on by default, 2026-04-08) and 317127 (disabled, 2026-06-15) [D43, D44, D45]
- First IME composition lost in Chromium 149+ on Windows (Chromium issue 523134891) · https://issues.chromium.org/issues/523134891 · state not readable (sign-in); the GitHub report that cites it was closed 2026-06-16 · criterion 6: same symptom as WebView2 #5625 [D35]
- IME support (Playwright #5777) · https://github.com/microsoft/playwright/issues/5777 · closed 2026-10-31 in a bulk closure, no composition API · criterion 6: how far the harness can automate composition [D77]

## Unknowns

- Whether spelling marks and suggestions appear in a Tauri 2.12.5 app on current WebView2 for a script-focused `contenteditable` (#5728 is open with no reply, runtime 153) [D24]. A prototype would have to focus the editor from script on start, type misspelled words, and record marks and the context menu in the real shell (P11).
- What WebView2 does for spelling beyond the facts above: which engine (Windows spell checker or Hunspell), whether it downloads dictionaries (network, G12), and which language list it uses with wry's `Language` from the Windows UI language. A prototype would have to watch network traffic on first use of an extra language and compare with the Windows language settings.
- Whether macOS WKWebView in a fresh Tauri app shows spelling marks without the `WebContinuousSpellCheckingEnabled` default, and whether a registered default (not a stored one) set before webview creation is enough: WebKit reads the key with `boolForKey:` once at first use, and the Tauri thread only shows `setBool:forKey:` and `defaults write` [D36, D37]. A prototype would have to run a bundled app with and without the key.
- Whether the macOS smart-quote/dash/text-replacement substitutions rewrite typed `"`, `'`, `--` and `(c)` in a `contenteditable` with `spellcheck="true"`, with default system settings; and whether `spellcheck="false"` or the `autocorrect` attribute stops it. This touches byte-stability (G5). The sources show the WebKit defaults and one Tauri report, not the combination [D36, D38, D40].
- Whether WebKitGTK, with wry's `set_enable_preedit(false)`, dispatches `compositionstart` and `compositionupdate` at all, and how GTK simple/compose dead-key sequences arrive (key events with the composed character, or composition events). The code reading above (wry sets preedit off [D1]; WebKit's confirm path dispatches `compositionend` [D40]) predicts `compositionend` without `compositionstart`; a prototype would have to log `keydown`, `compositionstart`, `compositionupdate`, `compositionend`, `beforeinput`, `input` with a compose or dead key and with fcitx5 and ibus.
- Whether Chromium/WebView2 on Windows dispatches composition events for dead keys today (Swiss German `^`, `¨`, `´`). The only primary statement is from 2016; UI Events says macOS and Linux use input methods [D63, D64]. A prototype would have to log events for `^` then `e` and `¨` then `u` in a Tauri window on Windows 11.
- Whether WebKit's composition event ordering flag (`InputMethodUsesCorrectKeyEventOrder`) state matches what WKWebView in each shipped macOS version does, and which macOS versions the Tauri build targets. The bugs show the flag was enabled on WebKit main in April 2026 and disabled in June 2026 [D43, D44].
- How Playwright's WebKit 27.2 differs from the WebKit in macOS and in WebKitGTK for the same inputs; the build Playwright uses for headless Linux runs is the WPE port, a different WebKit port from WebKitGTK, and `InputMethodFilter.cpp` has separate `PLATFORM(WPE)` code paths; how their input handling differs was not examined [D75, D50]. A prototype would have to run the same text-insertion test in Playwright WebKit and in the real shells and compare bytes.
- Whether `Input.imeSetComposition` works through WebView2's own CDP interface (`CallDevToolsProtocolMethod` [D33]) or through `connectOverCDP` against a Tauri window [D31], and whether the result matches what a TSF IME produces; and whether Playwright's CDP-injected composition reproduces the #5625 symptom. A prototype would have to run the same sequence both ways in the real shell on Windows.
- Whether text inside `contenteditable="false"` chips nested in an editing host is checked by each engine (the HTML Standard says only text in editing hosts and editable elements is checkable [D62]).
- Whether the Tauri 3 line changes the webview picture: `tauri` 3.0.0-alpha.5 (2026-10-21) and `tauri-runtime-cef` 3.0.0-alpha.6 (2026-10-14, "Tauri runtime interface for Chromium Embedded Framework") exist on crates.io; G2 names the Tauri 2 webviews, so I did not research them [D8].

## Sources

- D1 · https://docs.rs/crate/wry/0.57.1/source/ (crate tarball https://static.crates.io/crates/wry/wry-0.57.1.crate; files `src/lib.rs`, `src/webview2/mod.rs`, `src/webkitgtk/mod.rs`, `src/wkwebview/class/wry_web_view.rs`, `src/wkwebview/class/wry_web_view_parent.rs`) · wry 0.57.1 source · read 2026-10-31
- D2 · https://docs.rs/crate/wry/0.57.1/source/CHANGELOG.md · wry CHANGELOG (0.29.0 entry, PR #955) · read 2026-10-31
- D3 · https://docs.rs/crate/tauri/2.12.5/source/ (crate tarball https://static.crates.io/crates/tauri/tauri-2.12.5.crate; `src/webview/webview_window.rs`, `src/webview/mod.rs`, `src/menu/menu.rs`) · tauri 2.12.5 source · read 2026-10-31
- D4 · https://docs.rs/crate/tauri-runtime-wry/2.12.3/source/ (`src/lib.rs`, `Cargo.toml`) · tauri-runtime-wry 2.12.3 source · read 2026-10-31
- D5 · https://docs.rs/crate/muda/0.20.0/source/src/items/predefined.rs · muda 0.20.0 source (`PredefinedMenuItemType`) · read 2026-10-31
- D6 · https://docs.rs/crate/tauri-utils/2.10.2/source/src/config.rs · tauri-utils 2.10.2 config (`additional_browser_args`) · read 2026-10-31
- D7 · https://docs.rs/crate/tao/0.37.2/source/src/platform_impl/windows/keyboard.rs · tao 0.37.2 `keyboard.rs` (line 177) · read 2026-10-31
- D8 · https://crates.io/api/v1/crates/tauri/versions (also `wry`, `tao`, `tauri-runtime-wry`, `tauri-runtime-cef`, `muda`) · crates.io version index and release dates · read 2026-10-31
- D9 · https://docs.rs/crate/webkit2gtk/2.0.1/source/src/auto/web_context.rs · webkit2gtk 2.0.1 `WebContextExt` · read 2026-10-31
- D10 · https://docs.rs/crate/webview2-com-sys/0.39.1/source/src/bindings.rs · webview2-com-sys 0.39.1 bindings (published 2026-03-11) · read 2026-10-31
- D11 · https://v2.tauri.app/reference/webview-versions/ · Tauri docs, Webview Versions · read 2026-10-31
- D12 · https://v2.tauri.app/develop/tests/webdriver/ · Tauri docs, WebDriver testing · read 2026-10-31
- D13 · https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/browser-features · Differences between Microsoft Edge and WebView2 (updated 2026-10-02) · read 2026-10-31
- D14 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/584 · Add ability to disable spell checking · read 2026-10-31
- D15 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/914 · [Request] API for handling spell-checking · read 2026-10-31
- D16 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/2695 · [Request] Set spell checking language via API · read 2026-10-31
- D17 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/3758 · API for Spell Checker · read 2026-10-31
- D18 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/4840 · [Feature]: Add option to change spellchecking method · read 2026-10-31
- D19 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5294 · [Problem/Bug]: Spellchecker ignores HTML `lang` attribute, uses only Environment `Language` setting · read 2026-10-31
- D20 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/4372 · [Problem/Bug]: Spellchecker no longer functions after application restart · read 2026-10-31
- D21 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/2865 · WebView2 sending typed form contents to nleditor.osi.office.net · read 2026-10-31
- D22 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/2876 · Spellcheck menu suddenly appears when clicking on a misspelled word · read 2026-10-31
- D23 · https://nvd.nist.gov/vuln/detail/CVE-2023-22880 (read through https://services.nvd.nist.gov/rest/json/cves/2.0?cveId=CVE-2023-22880) · NVD, CVE-2023-22880 (Zoom clients and WebView2 spellcheck service) · read 2026-10-31
- D24 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5728 · [Problem/Bug]: Spellcheck never runs in a contenteditable after programmatic focus, until the user refocuses it by mouse · read 2026-10-31
- D25 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/2340 · Spell check suggestion text is not available in ContextMenuRequested event · read 2026-10-31
- D26 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/3142 · No spellcheck suggestion is displayed when ContextMenuRequested handler takes more than a few milliseconds · read 2026-10-31
- D27 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/3237 · Webview2 spellcheck suggestions empty for Dutch language; also https://github.com/MicrosoftEdge/WebView2Feedback/issues/3527 (Spell check correction from context menu doesn't always work) and https://github.com/MicrosoftEdge/WebView2Feedback/issues/3012 (Left click spellcheck menu causes WebView to lost focus) · read 2026-10-31
- D28 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5625 · WebView2 drops first CJK IME character & doubles Chinese punctuation in contentEditable (149.0.4022.69) · read 2026-10-31
- D29 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/1610 · Japanese / Chinese characters are getting duplicated while inputting using Japanese/Chinese IME Keyboard · read 2026-10-31
- D30 · https://github.com/MicrosoftEdge/WebView2Feedback/issues/5675 · [Problem/Bug]: IME composition window jumps to the top-left of the screen after the host window is resized · read 2026-10-31
- D31 · https://playwright.dev/docs/webview2 · Playwright, WebView2 · read 2026-10-31
- D32 · https://learn.microsoft.com/en-us/microsoft-edge/webview2/release-notes/sdk/1-0-4258-31 · WebView2 Release SDK 1.0.4258.31, for Runtime 154 (2026-09-28) · read 2026-10-31
- D33 · https://learn.microsoft.com/en-us/microsoft-edge/webview2/release-notes/sdk/1-0-4071-prerelease · WebView2 Prerelease SDK 1.0.4071 (2026-06-11), CDP calls through `CallDevToolsProtocolMethod` · read 2026-10-31
- D34 · https://github.com/chromium/chromium/blob/main/components/spellcheck/spellcheck_build_features.gni (also `components/spellcheck/browser/spellcheck_platform.h`, `chrome/browser/spellchecker/spellcheck_hunspell_dictionary.cc`) · Chromium spellcheck sources, main · read 2026-10-31
- D35 · https://github.com/ale-160/web-text/issues/2 · Bug Report: First IME Composition Input Lost in Chromium 149+ on Windows (third-party report; cites Chromium issue 523134891) · read 2026-10-31
- D36 · https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/mac/TextCheckerMac.mm · WebKit `TextCheckerMac.mm`, main · read 2026-10-31
- D37 · https://github.com/tauri-apps/tauri/issues/7705 · [bug] Safari MacOS Spelling is Hiding the Styles / Underline for Errors · read 2026-10-31
- D38 · https://github.com/tauri-apps/tauri/issues/12286 · [bug] When I input in the app, English double quotation marks will be changed to Chinese double quotation default · read 2026-10-31
- D39 · https://github.com/tauri-apps/tauri/issues/15924 · [bug] macOS: third-party IME drops the first keystroke inside the WebView (Safari with the same IME is unaffected) · read 2026-10-31
- D40 · https://github.com/WebKit/WebKit/blob/main/Source/WebCore/editing/Editor.cpp · WebKit `Editor.cpp`, main (`setComposition`, `confirmComposition`, `isSpellCheckingEnabledFor`, `markMisspellingsAfterTypingToWord`) · read 2026-10-31
- D41 · https://bugs.webkit.org/show_bug.cgi?id=167880 · WebKit bug 167880 (checked through https://bugs.webkit.org/rest/bug/167880) · read 2026-10-31
- D42 · https://bugs.webkit.org/show_bug.cgi?id=165004 · WebKit bug 165004, event order of keydown/keyup and composition events on macOS · read 2026-10-31
- D43 · https://bugs.webkit.org/show_bug.cgi?id=311717 · WebKit bug 311717, turn on correct composition event ordering by default · read 2026-10-31
- D44 · https://bugs.webkit.org/show_bug.cgi?id=317127 · WebKit bug 317127, Disable InputMethodUsesCorrectKeyEventOrder · read 2026-10-31
- D45 · https://bugs.webkit.org/show_bug.cgi?id=324380 · WebKit bug 324380, macOS Web Apps feature flag · read 2026-10-31
- D46 · https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/glib/WebKitWebContext.cpp · WebKitGTK `WebKitWebContext.cpp`, main · read 2026-10-31
- D47 · https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/glib/TextCheckerGLib.cpp · WebKit `TextCheckerGLib.cpp`, main · read 2026-10-31
- D48 · https://github.com/WebKit/WebKit/blob/main/Source/WebCore/platform/text/enchant/TextCheckerEnchant.cpp · WebKit `TextCheckerEnchant.cpp`, main · read 2026-10-31
- D49 · https://webkitgtk.org/reference/webkit2gtk/stable/method.WebContext.set_spell_checking_languages.html (and `set_spell_checking_enabled`) · WebKitGTK reference, WebContext spell checking · read 2026-10-31
- D50 · https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/glib/InputMethodFilter.cpp (and `API/gtk/WebKitInputMethodContextImplGtk.cpp`) · WebKit input method filter, main · read 2026-10-31
- D51 · https://docs.gtk.org/gtk3/method.IMContext.set_use_preedit.html · GTK 3, `gtk_im_context_set_use_preedit` · read 2026-10-31
- D52 · https://github.com/WebKit/WebKit/blob/main/Source/WebKit/UIProcess/API/glib/WebKitContextMenuActions.h.in · WebKitGTK context menu actions, main · read 2026-10-31
- D53 · https://github.com/tauri-apps/tauri/issues/11412 · [bug] IME window position appears out of input/textarea (cannot inline-input) on Tauri v2 apps in Linux · read 2026-10-31
- D54 · https://github.com/tauri-apps/tauri/issues/15859 · [bug] Tauri doesn't respect Compose key on Linux (KDE) · read 2026-10-31
- D55 · https://packages.debian.org/trixie/libwebkit2gtk-4.1-0 · Debian trixie package libwebkit2gtk-4.1-0, dependencies · read 2026-10-31
- D56 · https://github.com/tauri-apps/wry/issues/1908 · [Windows] IME composition/candidate UI loses caret positioning when switching between windows of the same application · read 2026-10-31
- D57 · https://github.com/tauri-apps/tauri/issues/16391 · [bug] [Windows] IME composition/candidate UI loses caret positioning when switching between windows of the same application · read 2026-10-31
- D58 · https://github.com/tauri-apps/tauri/issues/15436 · [bug] [Windows] WebView2 IME/TSF freezes on first focus of inputs with existing text (CJK input methods blocked) · read 2026-10-31
- D59 · https://github.com/tauri-apps/tao/issues/1408 · [Windows] Panic on a dead key: unwrap() on None in the WM_DEADCHAR branch of keyboard.rs · read 2026-10-31
- D60 · https://github.com/tauri-apps/tao/issues/1349 · [Windows] Backport tauri-apps/tao#1215 (reentrant input lock deadlock) to 0.35.x (and 0.34.x) for Tauri 2 users · read 2026-10-31
- D61 · https://github.com/tauri-apps/tauri/issues/8264 · [bug] Input Method does not work properly · read 2026-10-31
- D62 · https://html.spec.whatwg.org/multipage/interaction.html#spelling-and-grammar-checking · HTML Standard, spelling and grammar checking (Last Updated 30 October 2026) · read 2026-10-31
- D63 · https://w3c.github.io/uievents/ · UI Events, Editor's Draft 21 February 2026, section 4.3.2 Dead keys · read 2026-10-31
- D64 · https://lists.w3.org/Archives/Public/public-editing-tf/2016Jan/0029.html · Ryosuke Niwa, "Re: [UIEvents] Firing composition events for dead keys" (2016-01-09) · read 2026-10-31
- D65 · https://lists.w3.org/Archives/Public/public-webapps/2016JanMar/0035.html · Olli Pettay, same thread (2016-01-10) · read 2026-10-31
- D66 · https://developer.mozilla.org/en-US/docs/Web/API/Element/keydown_event · MDN, keydown event (last modified 2026-10-26) · read 2026-10-31
- D67 · https://github.com/codemirror/view/blob/main/src/input.ts (and `CHANGELOG.md`, 6.26.1 entry) · CodeMirror view source and changelog (blog-level source for engine quirks only) · read 2026-10-31
- D68 · https://github.com/ProseMirror/prosemirror-view/blob/master/src/input.ts (and `CHANGELOG.md`) · ProseMirror view source and changelog (blog-level source for engine quirks only) · read 2026-10-31
- D69 · https://discuss.codemirror.net/t/native-spellcheck-is-never-applied-to-a-codemirror-6-editing-host-on-chromium-152-electron-44/9886 · Native spellcheck is never applied to a CodeMirror 6 editing host on Chromium 152 (Electron 44) (2026-09-05 to 2026-09-09) · read 2026-10-31
- D70 · https://discuss.codemirror.net/t/os-level-spellcheck-is-disabled-on-ios-even-after-adding-contentattribute/4128 · OS-level spellcheck is disabled on iOS even after adding contentAttribute (2022-03-14) · read 2026-10-31
- D71 · https://playwright.dev/docs/release-notes · Playwright release notes 1.64 and 1.63 · read 2026-10-31
- D72 · playwright-core 1.64.1 `browsers.json` (npm package https://www.npmjs.com/package/playwright-core/v/1.64.1; read from the installed copy under `research/P10-prose-editor/app/node_modules/.pnpm/playwright-core@1.64.1/`) · Playwright browser revisions and versions · read 2026-10-31
- D73 · playwright-core 1.64.1 `lib/coreBundle.js` and `types/types.d.ts` (same copy as D72) · Playwright code: Chromium switches, `Input.insertText`, `Page.insertText`, `keyboard.insertText`, `newCDPSession` notes · read 2026-10-31
- D74 · https://playwright.dev/docs/browsers · Playwright, Browsers · read 2026-10-31
- D75 · https://github.com/microsoft/playwright/blob/main/browser_patches/webkit/pw_run.sh (and `packages/playwright-core/src/server/registry/index.ts`, `browser_patches/webkit/embedder/Playwright/`) · Playwright WebKit launch script, registry and embedder, main · read 2026-10-31
- D76 · https://github.com/ChromeDevTools/devtools-protocol/blob/master/json/browser_protocol.json · Chrome DevTools Protocol, Input domain (`imeSetComposition`, `insertText`) · read 2026-10-31
- D77 · https://github.com/microsoft/playwright/issues/5777 · [Feature] IME support · read 2026-10-31
- D78 · https://github.com/tauri-apps/wry/issues/1836 · Expose WebView2 AllowHostInputProcessing as a per-webview Windows option · read 2026-10-31
