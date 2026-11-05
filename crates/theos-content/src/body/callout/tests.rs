//! Unit tests of callout recognition and the voice and design title grammars.

use super::*;
use crate::body::{BlockKind, DesignNoteForm};
use crate::record::Record;
use crate::test_support::record;

fn at(r: &Record, span: Span) -> &str {
    span.get(r.text()).unwrap_or("<bad span>")
}

fn body(text: &str) -> Record {
    record("id: a\n", text)
}

fn voice(title: Option<&CalloutTitle>) -> &Voice {
    match title {
        Some(CalloutTitle::Voice(voice)) => voice,
        other => panic!("not a voice title: {other:?}"),
    }
}

// AC: CE-10.2
#[test]
fn voice_blocks_parse_with_their_title_tokens() {
    let r = body(
        "> [!voice] [[throat-kin]] · [[B-020]] · T2\n\
         > Sing a name the way it was first sung and they lean close to hear it.\n\
         > — a binder, teaching the naming-song\n",
    );
    let callouts = r.body().callouts();
    assert_eq!(callouts.len(), 1);
    let c = &callouts[0];
    assert_eq!(c.name, "voice");
    assert_eq!(at(&r, c.name_span), "voice");
    assert_eq!(c.depth, 1);
    assert_eq!(c.fold, None);
    assert_eq!(at(&r, c.title_span), "[[throat-kin]] · [[B-020]] · T2");
    assert!(at(&r, c.content_span).starts_with("> Sing a name"));
    assert_eq!(
        c.caption_span.map(|s| at(&r, s)),
        Some("— a binder, teaching the naming-song")
    );
    let v = voice(c.title.as_ref());
    assert_eq!(v.speaker.target, "throat-kin");
    assert_eq!(v.beliefs.len(), 1);
    assert_eq!(v.beliefs[0].target, "B-020");
    let tier = v.tier.as_ref().map(|t| (t.text.as_str(), at(&r, t.span)));
    assert_eq!(tier, Some(("T2", "T2")));
    assert_eq!(r.body().blocks()[0].kind, BlockKind::Callout(0));

    let r = body("> [!VOICE] [[a]]\n");
    assert_eq!(r.body().callouts()[0].name, "voice");
    assert_eq!(at(&r, r.body().callouts()[0].name_span), "VOICE");

    let r = body("> [!voice]- [[a]] · T1\n");
    let c = &r.body().callouts()[0];
    assert_eq!(c.fold, Some(Fold::Closed));
    assert_eq!(
        voice(c.title.as_ref()).tier.as_ref().map(|t| &*t.text),
        Some("T1")
    );

    let r = body("> [!voice]+ [[a]]\n");
    assert_eq!(r.body().callouts()[0].fold, Some(Fold::Open));

    let r = body("> [!voice] [[a]] · [[B-1]] · [[B-2]]\n");
    let v = voice(r.body().callouts()[0].title.as_ref());
    assert_eq!(v.beliefs.len(), 2);
    assert_eq!(v.tier, None);
}

#[test]
fn callouts_nest_and_continue_lazily() {
    let r = body("> [!note] n1 · open\n> > [!voice] [[a]]\n> > x\n");
    let callouts = r.body().callouts();
    assert_eq!(callouts.len(), 2);
    assert_eq!((callouts[0].name.as_str(), callouts[0].depth), ("note", 1));
    assert_eq!((callouts[1].name.as_str(), callouts[1].depth), ("voice", 2));
    assert_eq!(at(&r, callouts[1].title_span), "[[a]]");
    assert!(at(&r, callouts[1].content_span).contains('x'));
    assert_eq!(r.body().blocks().len(), 1);
    assert_eq!(r.body().blocks()[0].kind, BlockKind::Callout(0));

    let lazy = "> [!voice] [[a]]\n> x\ny\n-- a line\n";
    let r = body(lazy);
    let c = &r.body().callouts()[0];
    assert!(at(&r, c.content_span).contains("y\n-- a line"));
    assert_eq!(c.caption_span, None);

    let r = body(&format!("{lazy}— x\n"));
    assert_eq!(
        r.body().callouts()[0].caption_span.map(|s| at(&r, s)),
        Some("— x")
    );
}

// AC: CE-10.2
#[test]
fn design_notes_parse_with_their_title_tokens() {
    let r = body("> [!design] A remark about [[f04-choir-hall]]\n");
    let c = &r.body().callouts()[0];
    let Some(CalloutTitle::Design(design)) = &c.title else {
        panic!("not a design title: {:?}", c.title);
    };
    assert_eq!(
        design.text.as_ref().map(|t| t.text.as_str()),
        Some("A remark about [[f04-choir-hall]]")
    );
    assert_eq!(design.links.len(), 1);
    assert_eq!(design.links[0].target, "f04-choir-hall");
    let notes = r.body().design_notes();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].form, DesignNoteForm::Callout(0));
    assert_eq!(notes[0].span, c.span);

    let r = body("> [!design]\n");
    let Some(CalloutTitle::Design(design)) = &r.body().callouts()[0].title else {
        panic!("not a design title");
    };
    assert_eq!(design.text, None);
    assert_eq!(r.body().malformed(), []);
}

#[test]
fn other_callouts_keep_their_title() {
    let r = body("> [!tip] A tip\n");
    let c = &r.body().callouts()[0];
    assert_eq!(c.title, Some(CalloutTitle::Other));
    assert_eq!(c.name, "tip");
    assert_eq!(at(&r, c.title_span), "A tip");
}

// AC: CE-10.3
#[test]
fn malformed_voice_titles_report_their_line() {
    let forms = [
        "[!voice] T1",
        "[!voice] [[a]] · T1 · [[b]]",
        "[!voice] [[a]] ·  · T1",
        "[!voice] [[a]]·[[b]]",
        "[!voice] [[a]] • [[b]]",
        "[!voice] [[a]] | T1",
        "[!voice]",
    ];
    let text = forms.join("\n\n").replace("[!", "> [!") + "\n";
    let r = body(&text);
    let got: Vec<_> = r
        .body()
        .malformed()
        .iter()
        .map(|m| (m.kind.as_str(), m.position.line))
        .collect();
    let want: Vec<_> = (0..forms.len())
        .map(|i| ("callout-title", 4 + 2 * i))
        .collect();
    assert_eq!(got, want);
    assert_eq!(r.body().callouts().len(), forms.len());
    assert!(r.body().callouts().iter().all(|c| c.title.is_none()));
}

#[test]
fn voice_contents_are_exposed() {
    let r = body("> [!voice] [[hearth-folk]]\n> The hall sings.\n\n> [!voice] [[a]]\n> Second.\n");
    let c = &r.body().callouts()[0];
    assert_eq!(at(&r, c.content_span).trim_end(), "> The hall sings.");
}

#[test]
fn only_voice_callouts_have_captions() {
    for name in ["tip", "design"] {
        let r = body(&format!("> [!{name}] x\n> — y\n"));
        assert_eq!(r.body().callouts()[0].caption_span, None, "{name}");
    }
    let r = body("> [!voice] [[a]]\n> — y\n");
    assert_eq!(
        r.body().callouts()[0].caption_span.map(|s| at(&r, s)),
        Some("— y")
    );
}

fn note(title: Option<&CalloutTitle>) -> &Note {
    match title {
        Some(CalloutTitle::Note(note)) => note,
        other => panic!("not a note title: {other:?}"),
    }
}

// AC: CE-10.2
#[test]
fn note_states_are_recognised() {
    let r = body("> [!note] n3 · [[f04-terrain-v2]] · open\n");
    let n = note(r.body().callouts()[0].title.as_ref());
    assert_eq!((n.id.text.as_str(), n.number), ("n3", 3));
    assert_eq!(at(&r, n.id.span), "n3");
    assert_eq!(
        n.version.as_ref().map(|v| v.target.as_str()),
        Some("f04-terrain-v2")
    );
    assert_eq!(n.state, NoteState::Open);
    assert_eq!(at(&r, n.state_span), "open");

    let r = body("> [!note] n4 · resolved by [[fates-shrine-v5]]\n");
    let n = note(r.body().callouts()[0].title.as_ref());
    let NoteState::ResolvedBy(link) = &n.state else {
        panic!("not resolved: {:?}", n.state);
    };
    assert_eq!(link.target, "fates-shrine-v5");
    assert_eq!(n.version, None);
    assert_eq!(at(&r, n.state_span), "resolved by [[fates-shrine-v5]]");
    assert_eq!(n.state.spelling(), "resolved by [[fates-shrine-v5]]");

    let r = body("> [!note] n5 · dropped\n");
    let n = note(r.body().callouts()[0].title.as_ref());
    assert_eq!(n.state, NoteState::Dropped);
    assert_eq!(at(&r, n.state_span), "dropped");
    assert_eq!(n.state.spelling(), "dropped");

    let r = body("> [!note] n6 · closed\n");
    let n = note(r.body().callouts()[0].title.as_ref());
    assert_eq!(n.state, NoteState::Unknown("closed".to_owned()));
    assert_eq!(at(&r, n.state_span), "closed");
    assert_eq!(n.state.spelling(), "closed");

    let r = body("> [!note]- n2 · open\n");
    let c = &r.body().callouts()[0];
    assert_eq!(c.fold, Some(Fold::Closed));
    assert_eq!(note(c.title.as_ref()).number, 2);
    assert_eq!(r.body().malformed(), []);
}

// AC: CE-10.2
#[test]
fn note_text_leaves_out_title_and_caption() {
    let r = body(
        "> [!note] n3 · [[f04-terrain-v2]] · open\n\
         > Lakes cover a tenth of the floor; the valley should drain east, toward the rim.\n\
         > — Jann\n",
    );
    assert_eq!(
        note(r.body().callouts()[0].title.as_ref()).text,
        "Lakes cover a tenth of the floor; the valley should drain east, toward the rim."
    );

    let r = body("> [!note] n1 · open\n> a\n> b\nlazy\n>\n> c [[x|y]]\n> — Jann\n");
    assert_eq!(
        note(r.body().callouts()[0].title.as_ref()).text,
        "a b lazy\n\nc [[x|y]]"
    );

    let r = body("> [!note] n1 · open\n> — Jann\n");
    assert_eq!(note(r.body().callouts()[0].title.as_ref()).text, "");
}

// AC: CE-10.3
#[test]
fn malformed_note_titles_report_their_line() {
    let forms = [
        "[!note] open",
        "[!note] n1",
        "[!note] n1 · [[v]]",
        "[!note] n1 · resolved by",
        "[!note] n1 · open · extra",
        "[!note] n1·open",
    ];
    let text = forms.join("\n\n").replace("[!", "> [!") + "\n";
    let r = body(&text);
    let got: Vec<_> = r
        .body()
        .malformed()
        .iter()
        .map(|m| (m.kind.as_str(), m.position.line))
        .collect();
    let want: Vec<_> = (0..forms.len())
        .map(|i| ("callout-title", 4 + 2 * i))
        .collect();
    assert_eq!(got, want);
    assert!(r.body().callouts().iter().all(|c| c.title.is_none()));
}

// AC: CE-10.2
#[test]
fn whitespace_around_a_title_is_not_part_of_it() {
    let r = body("> [!voice] [[a]] · T1 \n> hi\n");
    let c = &r.body().callouts()[0];
    assert_eq!(at(&r, c.title_span), "[[a]] · T1");
    assert_eq!(
        voice(c.title.as_ref()).tier.as_ref().map(|t| &*t.text),
        Some("T1")
    );

    let r = body("> [!voice] [[a]] · T1  \n> hi\n");
    let c = &r.body().callouts()[0];
    assert_eq!(at(&r, c.title_span), "[[a]] · T1");
    assert!(c.title.is_some());

    let r = body("> [!note] n1 · open \n");
    let c = &r.body().callouts()[0];
    assert_eq!(note(c.title.as_ref()).state, NoteState::Open);
    assert_eq!(at(&r, c.title_span), "n1 · open");

    let r = body("> [!note]  n1 · open\n");
    let c = &r.body().callouts()[0];
    assert_eq!(at(&r, c.title_span), "n1 · open");
    assert_eq!(note(c.title.as_ref()).number, 1);

    let r = body("> [!design] Door width\t \n");
    let c = &r.body().callouts()[0];
    assert_eq!(at(&r, c.title_span), "Door width");
    let Some(CalloutTitle::Design(design)) = &c.title else {
        panic!("not a design title: {:?}", c.title);
    };
    assert_eq!(design.text.as_ref().map(|t| &*t.text), Some("Door width"));
    assert_eq!(r.body().malformed(), []);
}
