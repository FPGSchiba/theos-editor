# round-trip report

- commit: `6b22b2b32d8c3669f09c8fb170aaa2b4173a4c88`
- os: Windows_NT 10.0.26300 x64
- chromium: 156.0.8078.4
- webkit: 27.2
- date: 2026-11-03T17:50:21.734Z

`level` is `span` when the saved bytes are identical to the oracle, `block` when the mismatch stays
inside the edited block, and `none` otherwise. A row with an error ran no comparison.

## Summary

| variant | engine | unedited identical | edits matching | via api | via input | errors |
|---|---|---|---|---|---|---|
| codemirror | chromium | 17/17 | 60/60 | 30/30 | 30/30 | 0 |
| codemirror | webkit | 17/17 | 60/60 | 30/30 | 30/30 | 0 |
| prosemirror:best-effort | chromium | 17/17 | 57/60 | 29/30 | 28/30 | 0 |
| prosemirror:best-effort | webkit | 17/17 | 57/60 | 29/30 | 28/30 | 0 |
| prosemirror:stock | chromium | 5/17 | 8/60 | 4/30 | 4/30 | 0 |
| prosemirror:stock | webkit | 5/17 | 8/60 | 4/30 | 4/30 | 0 |

## Mismatches

### prosemirror:best-effort · chromium

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · api

- position: 23:1
- extra bytes: 1
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -20,6 +20,7 @@
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
```

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · input

- position: 23:1
- extra bytes: 1
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -20,6 +20,7 @@
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
```

#### `records/lore/truths/T-901.md` · `paste-html` · input

- position: 11:40
- extra bytes: 15
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -8,7 +8,7 @@
 requires:␣[T-012,␣T-001]
 cites:␣[E7]
 ---
-A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
+A␣synthetic␣paragraph␣before␣the␣lists.Pasted␣**rich**␣text.
 
 -␣first␣dash␣item
 -␣second␣dash␣item
```

### prosemirror:best-effort · webkit

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · api

- position: 23:1
- extra bytes: 1
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -20,6 +20,7 @@
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
```

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · input

- position: 23:1
- extra bytes: 1
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -20,6 +20,7 @@
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
```

#### `records/lore/truths/T-901.md` · `paste-html` · input

- position: 11:40
- extra bytes: 15
- level: block (the adapter says: span)

```diff
--- expected
+++ saved
@@ -8,7 +8,7 @@
 requires:␣[T-012,␣T-001]
 cites:␣[E7]
 ---
-A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
+A␣synthetic␣paragraph␣before␣the␣lists.Pasted␣**rich**␣text.
 
 -␣first␣dash␣item
 -␣second␣dash␣item
```

### prosemirror:stock · chromium

#### `records/lore/beliefs/B-901.md` · `external-while-dirty` · api

- position: 15:86
- extra bytes: 974
- level: none
- failed after: the edit

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣local␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `external-while-dirty` · input

- position: 15:86
- extra bytes: 974
- level: none
- failed after: the edit

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣local␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · api

- position: 15:86
- extra bytes: 998
- level: none

````diff
--- expected
+++ saved
@@ -12,30 +12,25 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -48,6 +43,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · input

- position: 15:86
- extra bytes: 998
- level: none

````diff
--- expected
+++ saved
@@ -12,30 +12,25 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -48,6 +43,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `undo-all` · api

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `undo-all` · input

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `unedited` · load

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/topics/synthetic-links-explaining.md` · `link-chip-delete` · api

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-chip-delete` · input

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-insert-popover` · api

- position: 12:69
- extra bytes: 398
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣[[T-012]]a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-insert-popover` · input

- position: 12:69
- extra bytes: 398
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣[[T-012]]a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `unedited` · load

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-voices-in-world.md` · `unedited` · load

- position: 16:72
- extra bytes: 878
- level: none

```diff
--- expected
+++ saved
@@ -13,8 +13,7 @@
 >␣The␣hall␣sings␣because␣the␣Fates␣are␣at␣their␣weaving.␣When␣the␣hum␣drops␣low,␣the␣Cutter␣is␣near,␣so␣we␣light␣three␣candles␣and␣keep␣the␣children␣in.
 
 >␣[!voice]␣[[throat-kin]]␣·␣[[B-020]]␣·␣T2
->␣Sing␣a␣name␣the␣way␣it␣was␣first␣sung␣and␣they␣lean␣close␣to␣hear␣it.
->␣Sing␣it␣wrong␣and␣you␣have␣only␣made␣noise␣in␣an␣empty␣hall.
+>␣Sing␣a␣name␣the␣way␣it␣was␣first␣sung␣and␣they␣lean␣close␣to␣hear␣it.␣Sing␣it␣wrong␣and␣you␣have␣only␣made␣noise␣in␣an␣empty␣hall.
 >␣—␣a␣binder,␣teaching␣the␣naming-song
 
 >␣[!voice]␣[[hearth-folk]]␣·␣[[B-014]]␣·␣T1
@@ -25,17 +24,16 @@
 >␣—␣a␣saying␣on␣the␣shrine␣stair
 
 >␣[!voice]␣[[mother-ilse]]␣·␣[[B-016]]␣·␣T1
->␣The␣Fates␣spin␣for␣those␣who␣climb.␣Hum␣with␣me,␣stair-child,
-and␣they␣will␣hear␣you.
+>␣The␣Fates␣spin␣for␣those␣who␣climb.␣Hum␣with␣me,␣stair-child,␣and␣they␣will␣hear␣you.
 
 >␣[!voice]␣[[throat-kin]]
 >␣A␣synthetic␣voice␣line␣with␣the␣word␣ledger␣in␣it.
 
 >␣[!voice]␣[[throat-kin]]
->␣--␣a␣line␣that␣starts␣with␣two␣hyphens␣is␣not␣a␣caption.
+>␣\--␣a␣line␣that␣starts␣with␣two␣hyphens␣is␣not␣a␣caption.
 >␣—␣a␣real␣caption␣after␣it
 
 >␣[!design]
 >␣A␣synthetic␣designer␣remark␣inside␣an␣in-world␣text.
 
-Design␣note:␣the␣player␣first␣meets␣this␣belief␣at␣the␣[[fates-shrine|Fates'␣shrine]]␣on␣F4,␣before␣any␣clue␣to␣T-012␣is␣confirmed.
+Design␣note:␣the␣player␣first␣meets␣this␣belief␣at␣the␣[[fates-shrine|Fates'␣shrine]]␣on␣F4,␣before␣any␣clue␣to␣T-012␣is␣confirmed.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `after-thematic-break` · api

- position: 13:1
- extra bytes: 856
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣synthetic␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `after-thematic-break` · input

- position: 13:1
- extra bytes: 856
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣synthetic␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `paste-html` · api

- position: 13:1
- extra bytes: 846
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `paste-html` · input

- position: 11:40
- extra bytes: 869
- level: none

```diff
--- expected
+++ saved
@@ -8,51 +8,51 @@
 requires:␣[T-012,␣T-001]
 cites:␣[E7]
 ---
-A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
+A␣synthetic␣paragraph␣before␣the␣lists.Pasted␣**rich**␣text.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `unedited` · load

- position: 13:1
- extra bytes: 846
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `note-state-change` · api

- position: 15:59
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣resolved␣by␣[[f04-terrain-v1]]
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `note-state-change` · input

- position: 15:59
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣resolved␣by␣[[f04-terrain-v1]]
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `unedited` · load

- position: 15:59
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣open
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-from-resolved` · api

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-from-resolved` · input

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-to-dropped` · api

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-to-dropped` · input

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `unedited` · load

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-add` · api

- position: 24:10
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -21,4 +21,4 @@
 at::␣[[fates-shrine]]
 with::␣[[mother-ilse]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-add` · input

- position: 24:10
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -21,4 +21,4 @@
 at::␣[[fates-shrine]]
 with::␣[[mother-ilse]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-change` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-change` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `body-start-backspace` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `body-start-backspace` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `paste-body-start` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `paste-body-start` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `unedited` · load

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `voice-end-sentence` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `voice-end-sentence` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-bare` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-bare` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-no-dash` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 at::␣[[fates-shrine]]
 when::␣T3
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-no-dash` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 at::␣[[fates-shrine]]
 when::␣T3
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-order` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-order` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-change-gap` · api

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-change-gap` · input

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `unedited` · load

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `combining-insert` · api

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `combining-insert` · input

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `emoji-insert-after` · api

- position: 16:55
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `emoji-insert-after` · input

- position: 16:55
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `nbsp-delete` · api

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `nbsp-delete` · input

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `unedited` · load

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `crlf-word-insert` · api

- position: 8:38
- extra bytes: 123
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣synthetic␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣synthetic␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `crlf-word-insert` · input

- position: 8:38
- extra bytes: 123
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣synthetic␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣synthetic␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `unedited` · load

- position: 8:38
- extra bytes: 113
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `space-at-paragraph-end` · api

- position: 9:60
- extra bytes: 248
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.␣
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `space-at-paragraph-end` · input

- position: 9:60
- extra bytes: 248
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.␣
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `unedited` · load

- position: 9:60
- extra bytes: 247
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `unedited` · load

- position: 10:87
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-delete` · api

- position: 10:75
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-delete` · input

- position: 10:75
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-insert` · api

- position: 10:92
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-insert` · input

- position: 10:92
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
\ No newline at end of file
```

### prosemirror:stock · webkit

#### `records/lore/beliefs/B-901.md` · `external-while-dirty` · api

- position: 15:86
- extra bytes: 974
- level: none
- failed after: the edit

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣local␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `external-while-dirty` · input

- position: 15:86
- extra bytes: 974
- level: none
- failed after: the edit

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣external␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣local␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · api

- position: 15:86
- extra bytes: 998
- level: none

````diff
--- expected
+++ saved
@@ -12,30 +12,25 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -48,6 +43,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `paste-crlf` · input

- position: 15:86
- extra bytes: 998
- level: none

````diff
--- expected
+++ saved
@@ -12,30 +12,25 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
 A␣paragraph␣under␣the␣first␣heading.␣First␣pasted␣line.
+
 Second␣pasted␣line.
 
 ##␣Synthetic␣heading␣level␣two
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -48,6 +43,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `undo-all` · api

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `undo-all` · input

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/beliefs/B-901.md` · `unedited` · load

- position: 15:86
- extra bytes: 959
- level: none

````diff
--- expected
+++ saved
@@ -12,10 +12,7 @@
 
 topic:␣synthetic-voices-in-world
 ---
-The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a
-grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts
-[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right
-behaviour.
+The␣binders␣treat␣every␣Listener␣as␣an␣ancestor:␣when␣one␣answers␣a␣song,␣they␣hear␣a␣grandmother␣recognising␣her␣own␣name.␣The␣belief␣distorts␣[[T-012|Listeners␣are␣the␣unwoven␣Tuners]],␣and␣it␣survives␣because␣it␣predicts␣the␣right␣behaviour.
 
 #␣Synthetic␣heading␣level␣one
 
@@ -25,16 +22,13 @@
 
 ###␣Synthetic␣heading␣level␣three
 
-Synthetic␣setext␣heading
-========================
+#␣Synthetic␣setext␣heading
 
 A␣paragraph␣under␣the␣setext␣heading.
 
-Another␣setext␣heading
-----------------------
+##␣Another␣setext␣heading
 
-Emphasis␣with␣*stars*␣and␣_underscores_,␣strong␣text␣with␣**stars**␣and␣__underscores__,
-and␣inline␣code␣like␣`a␣synthetic␣span`.
+Emphasis␣with␣*stars*␣and␣*underscores*,␣strong␣text␣with␣**stars**␣and␣**underscores**,␣and␣inline␣code␣like␣`a␣synthetic␣span`.
 
 ```text
 a␣synthetic␣fenced␣code␣block
@@ -47,6 +41,6 @@
 
 An␣autolink␣<https://example.com/p10>␣and␣a␣[Markdown␣link](https://example.com/p10/link).
 
-A␣line␣ended␣by␣two␣spaces␣␣
+A␣line␣ended␣by␣two␣spaces\
 continues␣on␣the␣next␣line,␣and␣a␣line␣ended␣by␣a␣backslash\
-continues␣on␣the␣one␣after␣it.
+continues␣on␣the␣one␣after␣it.
\ No newline at end of file
````

#### `records/lore/topics/synthetic-links-explaining.md` · `link-chip-delete` · api

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-chip-delete` · input

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-insert-popover` · api

- position: 12:69
- extra bytes: 398
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣[[T-012]]a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `link-insert-popover` · input

- position: 12:69
- extra bytes: 398
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣[[T-012]]a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-links-explaining.md` · `unedited` · load

- position: 12:69
- extra bytes: 389
- level: none

```diff
--- expected
+++ saved
@@ -9,14 +9,12 @@
 ---
 A␣link␣to␣a␣record:␣[[fates-shrine]].␣Links␣next␣to␣punctuation:␣([[B-020]]),␣[[C-031]];␣"[[T-012]]"?␣and␣[[mother-ilse]]!
 
-[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link
-to␣[[echo-vault]]
+[[hearth-folk]]␣starts␣this␣line,␣and␣the␣next␣line␣ends␣with␣a␣link␣to␣[[echo-vault]]
 
 The␣binders␣call␣them␣ancestors,␣but␣the␣operator-plate␣names␣a␣Listener.
 
-The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and
-[[fates-shrine|Fates'␣shrine]].
+The␣alias␣form␣shows␣its␣own␣words:␣[[B-020|Listeners␣answer␣to␣the␣right␣song]]␣and␣[[fates-shrine|Fates'␣shrine]].
 
 A␣dangling␣link␣points␣at␣a␣record␣that␣does␣not␣exist␣yet:␣[[song-cellar]].
 
-Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣_[[C-033|the␣operator-plate␣clue]]_.
+Emphasis␣around␣links:␣*[[fates-shrine]]*,␣**[[B-020]]**␣and␣*[[C-033|the␣operator-plate␣clue]]*.
\ No newline at end of file
```

#### `records/lore/topics/synthetic-voices-in-world.md` · `unedited` · load

- position: 16:72
- extra bytes: 878
- level: none

```diff
--- expected
+++ saved
@@ -13,8 +13,7 @@
 >␣The␣hall␣sings␣because␣the␣Fates␣are␣at␣their␣weaving.␣When␣the␣hum␣drops␣low,␣the␣Cutter␣is␣near,␣so␣we␣light␣three␣candles␣and␣keep␣the␣children␣in.
 
 >␣[!voice]␣[[throat-kin]]␣·␣[[B-020]]␣·␣T2
->␣Sing␣a␣name␣the␣way␣it␣was␣first␣sung␣and␣they␣lean␣close␣to␣hear␣it.
->␣Sing␣it␣wrong␣and␣you␣have␣only␣made␣noise␣in␣an␣empty␣hall.
+>␣Sing␣a␣name␣the␣way␣it␣was␣first␣sung␣and␣they␣lean␣close␣to␣hear␣it.␣Sing␣it␣wrong␣and␣you␣have␣only␣made␣noise␣in␣an␣empty␣hall.
 >␣—␣a␣binder,␣teaching␣the␣naming-song
 
 >␣[!voice]␣[[hearth-folk]]␣·␣[[B-014]]␣·␣T1
@@ -25,17 +24,16 @@
 >␣—␣a␣saying␣on␣the␣shrine␣stair
 
 >␣[!voice]␣[[mother-ilse]]␣·␣[[B-016]]␣·␣T1
->␣The␣Fates␣spin␣for␣those␣who␣climb.␣Hum␣with␣me,␣stair-child,
-and␣they␣will␣hear␣you.
+>␣The␣Fates␣spin␣for␣those␣who␣climb.␣Hum␣with␣me,␣stair-child,␣and␣they␣will␣hear␣you.
 
 >␣[!voice]␣[[throat-kin]]
 >␣A␣synthetic␣voice␣line␣with␣the␣word␣ledger␣in␣it.
 
 >␣[!voice]␣[[throat-kin]]
->␣--␣a␣line␣that␣starts␣with␣two␣hyphens␣is␣not␣a␣caption.
+>␣\--␣a␣line␣that␣starts␣with␣two␣hyphens␣is␣not␣a␣caption.
 >␣—␣a␣real␣caption␣after␣it
 
 >␣[!design]
 >␣A␣synthetic␣designer␣remark␣inside␣an␣in-world␣text.
 
-Design␣note:␣the␣player␣first␣meets␣this␣belief␣at␣the␣[[fates-shrine|Fates'␣shrine]]␣on␣F4,␣before␣any␣clue␣to␣T-012␣is␣confirmed.
+Design␣note:␣the␣player␣first␣meets␣this␣belief␣at␣the␣[[fates-shrine|Fates'␣shrine]]␣on␣F4,␣before␣any␣clue␣to␣T-012␣is␣confirmed.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `after-thematic-break` · api

- position: 13:1
- extra bytes: 856
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣synthetic␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `after-thematic-break` · input

- position: 13:1
- extra bytes: 856
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣synthetic␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `paste-html` · api

- position: 13:1
- extra bytes: 846
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `paste-html` · input

- position: 11:40
- extra bytes: 869
- level: none

```diff
--- expected
+++ saved
@@ -8,51 +8,51 @@
 requires:␣[T-012,␣T-001]
 cites:␣[E7]
 ---
-A␣synthetic␣paragraph␣before␣the␣lists.␣Pasted␣rich␣text.
+A␣synthetic␣paragraph␣before␣the␣lists.Pasted␣**rich**␣text.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/lore/truths/T-901.md` · `unedited` · load

- position: 13:1
- extra bytes: 846
- level: none

```diff
--- expected
+++ saved
@@ -10,49 +10,49 @@
 ---
 A␣synthetic␣paragraph␣before␣the␣lists.
 
--␣first␣dash␣item
--␣second␣dash␣item
-␣␣-␣a␣nested␣dash␣item
-␣␣␣␣-␣a␣doubly␣nested␣dash␣item
--␣third␣dash␣item
+*␣first␣dash␣item
+*␣second␣dash␣item
+␣␣*␣a␣nested␣dash␣item
+␣␣␣␣*␣a␣doubly␣nested␣dash␣item
+*␣third␣dash␣item
 
+
 *␣first␣star␣item
 *␣second␣star␣item
 ␣␣*␣a␣nested␣star␣item
 
-+␣first␣plus␣item
-+␣second␣plus␣item
 
+*␣first␣plus␣item
+*␣second␣plus␣item
+
 1.␣first␣numbered␣item
 2.␣second␣numbered␣item
 3.␣third␣numbered␣item
 
-1)␣first␣parenthesis␣item
-2)␣second␣parenthesis␣item
 
+1.␣first␣parenthesis␣item
+2.␣second␣parenthesis␣item
+
+
 1.␣first␣item␣of␣a␣list␣of␣ones
-1.␣second␣item␣of␣a␣list␣of␣ones
-1.␣third␣item␣of␣a␣list␣of␣ones
+2.␣second␣item␣of␣a␣list␣of␣ones
+3.␣third␣item␣of␣a␣list␣of␣ones
 
 A␣synthetic␣paragraph␣between␣two␣lists.
 
 1.␣an␣ordered␣item␣with␣a␣nested␣list
-␣␣␣-␣a␣dash␣item␣nested␣under␣it
-␣␣␣-␣another␣dash␣item
+␣␣␣*␣a␣dash␣item␣nested␣under␣it
+␣␣␣*␣another␣dash␣item
 2.␣the␣next␣ordered␣item
 
->␣A␣plain␣block␣quote␣that␣wraps␣over
->␣two␣lines␣of␣text.
+>␣A␣plain␣block␣quote␣that␣wraps␣over␣two␣lines␣of␣text.
 
-|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|
-|␣---␣|␣:---:␣|␣---:␣|
-|␣a1␣|␣b1␣|␣c1␣|
-|␣a2␣|␣b2␣|␣c2␣|
+|␣Column␣A␣|␣Column␣B␣|␣Column␣C␣|␣|␣---␣|␣:---:␣|␣---:␣|␣|␣a1␣|␣b1␣|␣c1␣|␣|␣a2␣|␣b2␣|␣c2␣|
 
 ---
 
 A␣paragraph␣after␣the␣dash␣thematic␣break.
 
-***
+---
 
-A␣paragraph␣after␣the␣star␣thematic␣break.
+A␣paragraph␣after␣the␣star␣thematic␣break.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `note-state-change` · api

- position: 15:59
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣resolved␣by␣[[f04-terrain-v1]]
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `note-state-change` · input

- position: 15:59
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣resolved␣by␣[[f04-terrain-v1]]
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/f04-terrain.md` · `unedited` · load

- position: 15:59
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -12,4 +12,4 @@
 ##␣Notes
 
 >␣[!note]␣n1␣·␣[[f04-terrain-v2]]␣·␣open
->␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
+>␣Too␣much␣relief␣for␣the␣Hearth;␣keep␣the␣floor␣walkable.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-from-resolved` · api

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-from-resolved` · input

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-to-dropped` · api

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `note-state-to-dropped` · input

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/maps/synthetic-notes.md` · `unedited` · load

- position: 27:45
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -24,4 +24,4 @@
 >␣A␣note␣that␣was␣dropped.
 
 >␣[!note]␣n5␣·␣[[f04-terrain-v1]]␣·␣dropped
->␣A␣note␣about␣one␣version␣that␣was␣dropped.
+>␣A␣note␣about␣one␣version␣that␣was␣dropped.
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-add` · api

- position: 24:10
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -21,4 +21,4 @@
 at::␣[[fates-shrine]]
 with::␣[[mother-ilse]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-add` · input

- position: 24:10
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -21,4 +21,4 @@
 at::␣[[fates-shrine]]
 with::␣[[mother-ilse]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-change` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `beat-param-change` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `body-start-backspace` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `body-start-backspace` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `paste-body-start` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `paste-body-start` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `unedited` · load

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `voice-end-sentence` · api

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-004.md` · `voice-end-sentence` · input

- position: 23:10
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -20,4 +20,4 @@
 ##␣Beat␣2␣—␣The␣murmur
 at::␣[[fates-shrine]]
 reveals::␣[[C-031]]
-when::␣T1
+when::␣T1
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-bare` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-bare` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-no-dash` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 at::␣[[fates-shrine]]
 when::␣T3
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-no-dash` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 at::␣[[fates-shrine]]
 when::␣T3
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-order` · api

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-add-order` · input

- position: 43:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -40,4 +40,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-change-gap` · api

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `beat-param-change-gap` · input

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/story/scripts/S-901.md` · `unedited` · load

- position: 42:79
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -39,4 +39,4 @@
 ##␣Beat␣5␣-␣A␣synthetic␣heading␣without␣an␣em␣dash
 at::␣[[fates-shrine]]
 
-Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
+Ilse␣keeps␣the␣vigil␣and␣tells␣the␣player␣why␣the␣hall␣must␣never␣fall␣silent.
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `combining-insert` · api

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `combining-insert` · input

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `emoji-insert-after` · api

- position: 16:55
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `emoji-insert-after` · input

- position: 16:55
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂␣done
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `nbsp-delete` · api

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `nbsp-delete` · input

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-characters.md` · `unedited` · load

- position: 16:50
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -13,4 +13,4 @@
 
 A␣non-breaking␣space␣joins␣5⍽km␣into␣one␣unit.
 
-An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
+An␣emoji,␣which␣is␣a␣surrogate␣pair␣in␣UTF-16:␣🙂
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `crlf-word-insert` · api

- position: 8:38
- extra bytes: 123
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣synthetic␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣synthetic␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `crlf-word-insert` · input

- position: 8:38
- extra bytes: 123
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣synthetic␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣synthetic␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-crlf.md` · `unedited` · load

- position: 8:38
- extra bytes: 113
- level: none

```diff
--- expected
+++ saved
@@ -5,8 +5,7 @@
 title:␣Synthetic␣CRLF␣file
 type:␣document
 ---
-A␣synthetic␣paragraph␣that␣wraps␣over
-two␣lines␣to␣test␣line␣ends.
+A␣synthetic␣paragraph␣that␣wraps␣over␣two␣lines␣to␣test␣line␣ends.
 
 >␣[!voice]␣[[throat-kin]]
->␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
+>␣A␣synthetic␣voice␣line␣in␣a␣file␣with␣CRLF␣line␣ends.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `space-at-paragraph-end` · api

- position: 9:60
- extra bytes: 248
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.␣
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `space-at-paragraph-end` · input

- position: 9:60
- extra bytes: 248
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.␣
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/things/synthetic-whitespace.md` · `unedited` · load

- position: 9:60
- extra bytes: 247
- level: none

```diff
--- expected
+++ saved
@@ -6,17 +6,13 @@
 type:␣document
 where:␣[fates-shrine]
 ---
-A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣
-and␣a␣tab⇥inside␣its␣second␣line.
+A␣paragraph␣with␣a␣single␣trailing␣space␣on␣its␣first␣line␣and␣a␣tab⇥inside␣its␣second␣line.
 
-A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.␣␣␣
+A␣paragraph␣whose␣last␣line␣ends␣with␣three␣spaces.
 
-
 Two␣blank␣lines␣sit␣above␣this␣paragraph.
 
-
-
 Three␣blank␣lines␣sit␣above␣this␣paragraph.
 
-A␣line␣ended␣by␣two␣spaces␣␣
-is␣a␣hard␣break␣that␣must␣survive.
+A␣line␣ended␣by␣two␣spaces\
+is␣a␣hard␣break␣that␣must␣survive.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `unedited` · load

- position: 10:87
- extra bytes: 1
- level: none

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-delete` · api

- position: 10:75
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-delete` · input

- position: 10:75
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
+A␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-insert` · api

- position: 10:92
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
\ No newline at end of file
```

#### `records/world/hearth/f04-choir-hall.md` · `word-insert` · input

- position: 10:92
- extra bytes: 1
- level: block

```diff
--- expected
+++ saved
@@ -7,4 +7,4 @@
 number:␣4
 status:␣draft
 ---
-A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
+A␣half-buried␣resonating␣chamber␣the␣Hearth␣folk␣have␣turned␣into␣their␣very␣holiest␣place.
\ No newline at end of file
```

Every row is in the JSON file next to this one.
