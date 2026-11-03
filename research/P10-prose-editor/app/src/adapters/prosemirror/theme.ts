import 'prosemirror-view/style/prosemirror.css';

const STYLE = `
.pm-write, .pm-source-mode { box-sizing: border-box; min-height: 12em; padding: 12px 14px; outline: none;
  color: var(--textProse); background: var(--panel); border: 1px solid var(--border); caret-color: var(--text);
  font: 400 15px/24px var(--font-ui); }
.pm-write:focus, .pm-source-mode:focus { border-color: var(--borderHi); }
.pm-write p { margin: 0 0 12px; }
.pm-write h1 { margin: 8px 0; color: var(--text); font: 600 26px/32px var(--font-ui); letter-spacing: -0.3px; }
.pm-write h2 { margin: 8px 0; color: var(--text); font: 600 19px/26px var(--font-ui); letter-spacing: -0.1px; }
.pm-write h3, .pm-write h4, .pm-write h5, .pm-write h6 { margin: 8px 0; color: var(--text); font: 600 15px/24px var(--font-ui); }
.pm-write strong { font-weight: 700; color: var(--text); }
.pm-write blockquote { margin: 0 0 12px; padding-left: 12px; border-left: 2px solid var(--border); }
.pm-write .hx-callout p, .pm-write .tu-voice p { margin: 0; }
.pm-callout-head { user-select: none; }
.pm-callout-label { font: 700 10.5px/16px var(--font-mono); letter-spacing: 0.4px; text-transform: uppercase; color: var(--textDim); }
.pm-callout-title { display: none; }
.pm-callout-title.is-raw { display: block; margin: 0; font: 400 12px/20px var(--font-mono); color: var(--textMute); white-space: pre-wrap; }
.has-raw-title > .pm-callout-head { display: none; }
.pm-callout-title:not(.is-raw) + * { margin-top: 0; }
.pm-write .pm-beat-heading > .pm-beat-text { display: none; }
.pm-write .pm-beat-heading.is-raw > .pm-beat-text, .pm-write .pm-beat-heading.is-bare > .pm-beat-text { display: inline; }
.pm-write .pm-beat-heading.is-raw > .pm-beat-shown, .pm-write .pm-beat-heading.is-bare > .pm-beat-shown { display: none; }
.pm-beat-shown { user-select: none; }
.pm-write .pm-beat-attrs > .pm-beat-lines { display: none; }
.pm-write .pm-beat-attrs.is-raw > .pm-beat-lines { display: block; margin: 0 0 12px; font: 400 12px/20px var(--font-mono); color: var(--textMute); white-space: pre-wrap; }
.pm-write .pm-beat-attrs.is-raw > .pm-beat-row { display: none; }
.pm-beat-row { margin: 0 0 12px; }
.pm-beat-row:empty { display: none; }
.pm-marker { color: var(--textMute); font-weight: 400; font-style: normal; }
.pm-source-mode { font: 400 13px/20px var(--font-mono); }
.pm-source { margin: 0; white-space: pre-wrap; font: inherit; }
.pm-source code { font: inherit; }
.pm-frontmatter { color: var(--textMute); }
`;

/** Install the look of the editor once per page (a stylesheet of the page, not a child of the editor). */
export function installTheme(): void {
  if (document.getElementById('p10-prosemirror-theme') !== null) return;
  const style = document.createElement('style');
  style.id = 'p10-prosemirror-theme';
  style.textContent = STYLE;
  document.head.append(style);
}
