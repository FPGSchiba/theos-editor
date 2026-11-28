//! The Codex views of the editor (P11 §5.8, §5.10) on the synthetic `editor-codex` fixture.

use std::path::Path;

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use theos_studio::Workspace;
use theos_studio::codex_view::{
    self, FieldRow, FieldValue, RecordPage, TreeNode, TreeTab, ViewInline, ViewLink, ViewNode,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The fixture project.
fn fixture() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/projects/editor-codex"
    ))
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The fixture copied to a temporary folder and opened, so no cache lands in the repository.
fn workspace() -> Result<(TempDir, Workspace), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    copy_tree(fixture(), dir.path())?;
    let ws = Workspace::open(dir.path())?;
    Ok((dir, ws))
}

/// An empty project, opened.
fn empty_workspace() -> Result<(TempDir, Workspace), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("theos.toml"),
        "[project]\nname = \"synthetic-empty\"\nkind = \"content\"\nschema_version = 1\n",
    )?;
    let ws = Workspace::open(dir.path())?;
    Ok((dir, ws))
}

// --- tree -----------------------------------------------------------------------------------

/// The id of a record row, or the label of a group.
fn name(node: &TreeNode) -> &str {
    match node {
        TreeNode::Record { record, .. } => &record.id,
        TreeNode::Group { label, .. } => label,
    }
}

fn children(node: &TreeNode) -> &[TreeNode] {
    match node {
        TreeNode::Record { children, .. } | TreeNode::Group { children, .. } => children,
    }
}

fn tag(node: &TreeNode) -> Option<&str> {
    match node {
        TreeNode::Record { tag, .. } => tag.as_deref(),
        TreeNode::Group { .. } => None,
    }
}

fn names(nodes: &[TreeNode]) -> Vec<&str> {
    nodes.iter().map(name).collect()
}

fn find<'a>(nodes: &'a [TreeNode], id: &str) -> Result<&'a TreeNode, String> {
    nodes
        .iter()
        .find(|node| name(node) == id)
        .ok_or_else(|| format!("no `{id}` in {:?}", names(nodes)))
}

/// Every record row of the tree, depth first.
fn all_ids(nodes: &[TreeNode]) -> Vec<&str> {
    let mut ids = Vec::new();
    for node in nodes {
        if let TreeNode::Record { record, .. } = node {
            ids.push(record.id.as_str());
        }
        ids.extend(all_ids(children(node)));
    }
    ids
}

#[test]
fn where_tab_nests_tower_band_floor_location() -> TestResult {
    let (_dir, ws) = workspace()?;
    let tree = codex_view::tree(&ws, TreeTab::Where)?;
    let tower = find(&tree, "tower")?;
    let band = find(children(tower), "hearth")?;
    assert_eq!(tag(band), Some("1–20"));
    let floor = find(children(band), "f04-choir-hall")?;
    assert_eq!(tag(floor), Some("draft"));
    assert_eq!(
        names(children(floor)),
        ["reed-marsh", "fates-shrine"],
        "locations by number, then title; the loop locations sit at the top level"
    );
    Ok(())
}

#[test]
fn who_tab_nests_people_faction_npc() -> TestResult {
    let (_dir, ws) = workspace()?;
    let tree = codex_view::tree(&ws, TreeTab::Who)?;
    assert_eq!(names(&tree), ["hearth-folk"]);
    let people = find(&tree, "hearth-folk")?;
    let faction = find(children(people), "fates-cult")?;
    assert_eq!(names(children(faction)), ["mother-ilse"]);
    Ok(())
}

#[test]
fn lore_tab_groups_by_kind_sorted_by_id() -> TestResult {
    let (_dir, ws) = workspace()?;
    let tree = codex_view::tree(&ws, TreeTab::Lore)?;
    // No topic and no equation in the fixture: no empty group.
    assert_eq!(names(&tree), ["Truths", "Beliefs", "Clues"]);
    assert_eq!(names(children(find(&tree, "Truths")?)), ["T-012", "T-013"]);
    assert_eq!(names(children(find(&tree, "Beliefs")?)), ["B-016", "B-017"]);
    assert_eq!(names(children(find(&tree, "Clues")?)), ["C-031", "C-032"]);
    let never = find(children(find(&tree, "Truths")?), "T-013")?;
    match never {
        TreeNode::Record { record, .. } => assert!(record.mark.never),
        TreeNode::Group { .. } => return Err("T-013 is a group".into()),
    }
    Ok(())
}

#[test]
fn things_tab_has_only_registered_kinds() -> TestResult {
    let (dir, mut ws) = workspace()?;
    // `enemy` is not registered before P21: its record is left out, with no group.
    std::fs::create_dir_all(dir.path().join("things"))?;
    std::fs::write(
        dir.path().join("things/synthetic-lamp.md"),
        "---\nid: synthetic-lamp\nkind: thing\ntitle: Synthetic lamp\n---\nSynthetic text.\n",
    )?;
    std::fs::write(
        dir.path().join("things/synthetic-foe.md"),
        "---\nid: synthetic-foe\nkind: enemy\ntitle: Synthetic foe\n---\nSynthetic text.\n",
    )?;
    ws.index_mut().sync()?;
    let tree = codex_view::tree(&ws, TreeTab::Things)?;
    assert_eq!(names(&tree), ["Things"]);
    // The fixture's own thing, `synthetic-html`, sorts before the lamp.
    assert_eq!(all_ids(&tree), ["synthetic-html", "synthetic-lamp"]);
    Ok(())
}

#[test]
fn dangling_parent_and_cycle_sit_at_top_level_once() -> TestResult {
    let (_dir, ws) = workspace()?;
    let tree = codex_view::tree(&ws, TreeTab::Where)?;
    let top = names(&tree);
    for id in ["synthetic-stray", "synthetic-loop-a", "synthetic-loop-b"] {
        assert!(top.contains(&id), "{id} not at the top level: {top:?}");
    }
    let mut ids = all_ids(&tree);
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        count,
        "a record shows twice: {:?}",
        all_ids(&tree)
    );
    for id in ["synthetic-loop-a", "synthetic-loop-b"] {
        assert!(
            children(find(&tree, id)?).is_empty(),
            "{id} nests its cycle mate"
        );
    }
    Ok(())
}

#[test]
fn empty_project_gives_empty_tabs() -> TestResult {
    let (_dir, ws) = empty_workspace()?;
    for tab in [TreeTab::Where, TreeTab::Who, TreeTab::Lore, TreeTab::Things] {
        assert!(codex_view::tree(&ws, tab)?.is_empty(), "{tab:?}");
    }
    Ok(())
}

// --- record ---------------------------------------------------------------------------------

fn floor(ws: &Workspace) -> Result<RecordPage, theos_studio::StudioError> {
    codex_view::record(ws, "f04-choir-hall")
}

fn links_of(inlines: &[ViewInline]) -> Vec<&ViewLink> {
    let mut links = Vec::new();
    for inline in inlines {
        match inline {
            ViewInline::Link(link) => links.push(link),
            ViewInline::Emphasis { inlines } | ViewInline::Strong { inlines } => {
                links.extend(links_of(inlines));
            }
            _ => {}
        }
    }
    links
}

/// The kind of each block, as its serde tag.
fn block_kinds(nodes: &[ViewNode]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    nodes
        .iter()
        .map(|node| {
            let json = serde_json::to_value(node)?;
            Ok(json["type"].as_str().ok_or("no type")?.to_owned())
        })
        .collect()
}

// AC: CE-33.1
#[test]
fn floor_header_names_kind_parent_and_status() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    assert_eq!(page.header.record.id, "f04-choir-hall");
    assert_eq!(page.header.record.title.as_deref(), Some("The Choir-Hall"));
    assert_eq!(
        page.header.record.path.as_deref(),
        Some("world/hearth/f04-choir-hall.md")
    );
    assert_eq!(page.header.record.kind.as_deref(), Some("floor"));
    assert_eq!(page.header.record.status.as_deref(), Some("draft"));
    let parent = page.header.parent.as_ref().ok_or("no parent")?;
    assert_eq!(parent.title.as_deref(), Some("Hearth"));
    // The two locations; the loop locations name each other as parent.
    assert_eq!(page.header.children, 2);
    assert_eq!(
        usize::try_from(page.header.links_in)?,
        page.links_in.len(),
        "the count is the backlinks"
    );
    Ok(())
}

// AC: CE-33.1
#[test]
fn floor_body_is_a_typed_block_tree() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    assert_eq!(
        block_kinds(&page.body)?,
        [
            "paragraph",
            "heading",
            "list",
            "voice",
            "design",
            "paragraph",
            "raw",
            "quote"
        ]
    );
    let ViewNode::Paragraph { inlines } = &page.body[0] else {
        return Err("the first block is no paragraph".into());
    };
    let chips = links_of(inlines);
    assert_eq!(chips.len(), 2);
    let shrine = chips[0].target.as_ref().ok_or("fates-shrine dangles")?;
    assert_eq!(shrine.title.as_deref(), Some("The Fates' shrine"));
    assert_eq!(shrine.kind.as_deref(), Some("location"));
    assert_eq!(chips[1].id, "reed-marsh");
    assert_eq!(chips[1].shown.as_deref(), Some("marsh"));
    assert_eq!(
        chips[1]
            .target
            .as_ref()
            .ok_or("reed-marsh dangles")?
            .kind
            .as_deref(),
        Some("location")
    );

    let ViewNode::List { items, .. } = &page.body[2] else {
        return Err("the third block is no list".into());
    };
    let ViewNode::Paragraph { inlines } = items
        .first()
        .and_then(|item| item.first())
        .ok_or("empty list")?
    else {
        return Err("the list item is no paragraph".into());
    };
    let lore = links_of(inlines);
    let clue = lore.first().ok_or("no lore chip")?;
    assert_eq!(clue.id, "C-031");
    assert_eq!(
        clue.target.as_ref().ok_or("C-031 dangles")?.kind.as_deref(),
        Some("clue")
    );

    let ViewNode::Voice {
        speaker,
        beliefs,
        tier,
        caption,
        ..
    } = &page.body[3]
    else {
        return Err("the fourth block is no voice".into());
    };
    assert_eq!(speaker.id, "mother-ilse");
    assert_eq!(
        speaker
            .target
            .as_ref()
            .ok_or("speaker dangles")?
            .title
            .as_deref(),
        Some("Mother Ilse")
    );
    assert_eq!(beliefs.first().ok_or("no belief")?.id, "B-016");
    assert_eq!(tier.as_deref(), Some("T1"));
    assert!(
        matches!(caption.as_deref(), Some([ViewInline::Text { text }]) if text == "a synthetic caption on the shrine stair"),
        "{:?}",
        serde_json::to_value(caption)?
    );

    let ViewNode::Paragraph { inlines } = &page.body[5] else {
        return Err("the sixth block is no paragraph".into());
    };
    let dangling = links_of(inlines);
    let ghost = dangling.first().ok_or("no dangling chip")?;
    assert_eq!(ghost.id, "synthetic-dangling");
    assert!(ghost.target.is_none());
    // The unclosed link stays text.
    assert_eq!(dangling.len(), 1);

    let ViewNode::Raw { text } = &page.body[6] else {
        return Err("the seventh block is not raw".into());
    };
    assert!(text.starts_with("<div>"), "{text}");

    // A callout whose title does not parse, a block-level fault P02 reports, keeps its title
    // line as written; an in-line fault (the unclosed link above) stays text.
    let ViewNode::Quote { nodes } = &page.body[7] else {
        return Err("the malformed callout is no quote".into());
    };
    assert!(
        matches!(nodes.first(), Some(ViewNode::Raw { text }) if text == "[!voice] T1 · a synthetic voice title that names no speaker"),
        "{nodes:?}"
    );
    Ok(())
}

// AC: CE-33.1
#[test]
fn floor_lore_table_has_verbs_and_never() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    let rows: Vec<(&str, Option<&str>, Option<&str>)> = page
        .lore
        .iter()
        .map(|row| {
            (
                row.record.id.as_str(),
                row.verb.as_deref(),
                row.touches.as_ref().map(|t| t.id.as_str()),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("B-017", Some("distorts"), Some("T-013")),
            ("C-031", Some("hints"), Some("T-012")),
        ]
    );
    let never = page.lore[0].touches.as_ref().ok_or("no touches")?;
    assert!(never.mark.never);
    assert!(
        !page.lore[1]
            .touches
            .as_ref()
            .ok_or("no touches")?
            .mark
            .never
    );
    // The clue's gate is its tier; a belief has none.
    assert_eq!(page.lore[1].tier.as_deref(), Some("T1"));
    assert_eq!(page.lore[0].tier, None);
    Ok(())
}

// AC: CE-33.1
#[test]
fn pictures_are_the_concepts_for_the_record() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    assert_eq!(page.pictures.len(), 1);
    assert_eq!(page.pictures[0].concept.id, "f04-choir-hall-art");
    assert_eq!(page.pictures[0].picked, None);
    let shrine = codex_view::record(&ws, "fates-shrine")?;
    assert_eq!(shrine.pictures.len(), 1);
    assert_eq!(
        shrine.pictures[0].picked.as_deref(),
        Some("fates-shrine-art-v5")
    );
    // A record without a concept has no pictures.
    assert!(codex_view::record(&ws, "reed-marsh")?.pictures.is_empty());
    Ok(())
}

fn row<'a>(rows: &'a [FieldRow], key: &str) -> Result<&'a FieldValue, String> {
    rows.iter()
        .find(|row| row.key == key)
        .map(|row| &row.value)
        .ok_or_else(|| format!("no `{key}`"))
}

// AC: CE-33.1
#[test]
fn front_matter_rows_keep_file_order_and_types() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    let keys: Vec<&str> = page.front_matter.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["id", "kind", "title", "parent", "number", "status"]);
    let FieldValue::Link(parent) = row(&page.front_matter, "parent")? else {
        return Err("parent is no link".into());
    };
    assert_eq!(parent.id, "hearth");
    assert_eq!(
        parent
            .target
            .as_ref()
            .ok_or("parent dangles")?
            .title
            .as_deref(),
        Some("Hearth")
    );
    assert!(matches!(row(&page.front_matter, "number")?, FieldValue::Text(text) if text == "4"));

    // A tier field draws the tier selector: the clue's gate.
    let clue = codex_view::record(&ws, "C-031")?;
    assert!(matches!(row(&clue.front_matter, "gate")?, FieldValue::Tier(tier) if tier == "T1"));
    let FieldValue::List(places) = row(&clue.front_matter, "where")? else {
        return Err("where is no list".into());
    };
    assert!(matches!(places.as_slice(), [FieldValue::Link(link)] if link.id == "f04-choir-hall"));

    // A lens is a map whose entries keep their keys.
    let npc = codex_view::record(&ws, "mother-ilse")?;
    let FieldValue::Map(lens) = row(&npc.front_matter, "lens")? else {
        return Err("lens is no map".into());
    };
    assert_eq!(lens.len(), 1);
    assert_eq!(lens[0].key, "speaks_to");
    Ok(())
}

// AC: CE-33.1
#[test]
fn floor_findings_include_dangling_link() -> TestResult {
    let (_dir, ws) = workspace()?;
    let page = floor(&ws)?;
    let json = serde_json::to_value(&page.findings)?;
    let rules: Vec<&str> = json
        .as_array()
        .ok_or("findings")?
        .iter()
        .filter_map(|f| f["rule"].as_str())
        .collect();
    assert!(rules.contains(&"link/dangling"), "{rules:?}");
    Ok(())
}

fn relations_of<'a>(page: &'a RecordPage, source: &str) -> Option<Vec<&'a str>> {
    page.used_by
        .iter()
        .find(|used| used.source.id == source)
        .map(|used| used.relations.iter().map(String::as_str).collect())
}

fn hash(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    Ok(Sha256::digest(std::fs::read(path)?).to_vec())
}

// AC: CE-33.2
#[test]
fn used_by_is_derived() -> TestResult {
    let (dir, mut ws) = workspace()?;
    let file = dir.path().join("world/hearth/f04-choir-hall.md");
    let before = hash(&file)?;
    let page = floor(&ws)?;
    assert_eq!(relations_of(&page, "C-031"), Some(vec!["where"]));
    assert_eq!(
        relations_of(&page, "S-004"),
        Some(vec!["where", "in a beat"]),
        "each relation once"
    );
    assert_eq!(relations_of(&page, "fates-shrine"), Some(vec!["child of"]));
    assert_eq!(
        hash(&file)?,
        before,
        "the view writes nothing to the record"
    );

    std::fs::remove_file(dir.path().join("lore/clues/C-031.md"))?;
    ws.index_mut().sync()?;
    let page = floor(&ws)?;
    assert_eq!(relations_of(&page, "C-031"), None);
    assert!(page.lore.iter().all(|row| row.record.id != "C-031"));

    std::fs::write(
        dir.path().join("lore/beliefs/B-018.md"),
        "---\nid: B-018\nkind: belief\ntitle: A second synthetic belief\nstatus: draft\n\
         held_by: [fates-cult]\ndistorts: T-012\n---\nSynthetic text about [[f04-choir-hall]].\n",
    )?;
    ws.index_mut().sync()?;
    let page = floor(&ws)?;
    assert_eq!(relations_of(&page, "B-018"), Some(vec!["mentions"]));
    assert!(page.lore.iter().any(|row| row.record.id == "B-018"));
    assert_eq!(hash(&file)?, before);
    Ok(())
}

#[test]
fn relation_names() {
    for (slot, relation) in [
        ("field:found_at", "found at"),
        ("field:where", "where"),
        ("field:about", "about"),
        ("field:parent", "child of"),
        ("beat:at", "in a beat"),
        ("voice:speaker", "voice"),
        ("voice:belief", "voice"),
        ("lens:about", "lens"),
        ("note:version", "note"),
        ("note:resolved-by", "note"),
        ("mention", "mentions"),
    ] {
        assert_eq!(codex_view::relation(slot), relation, "{slot}");
    }
}

// --- query ----------------------------------------------------------------------------------

fn hit_ids(view: &codex_view::QueryView) -> Vec<&str> {
    let mut ids: Vec<&str> = view.hits.iter().map(|h| h.record.id.as_str()).collect();
    ids.sort_unstable();
    ids
}

// AC: CE-33.3
#[test]
fn query_id_prefix_finds_the_floor() -> TestResult {
    let (_dir, ws) = workspace()?;
    // The floor, and the concept for it, whose id starts with the floor's.
    let view = codex_view::query(&ws, "#f04", 20)?;
    assert_eq!(hit_ids(&view), ["f04-choir-hall", "f04-choir-hall-art"]);
    assert_eq!(view.hits[0].record.kind.as_deref(), Some("floor"));
    assert_eq!(view.note, None);
    // Case does not matter, and the prefixes combine.
    assert_eq!(
        hit_ids(&codex_view::query(&ws, "#F04 :floor", 20)?),
        ["f04-choir-hall"]
    );
    Ok(())
}

// AC: CE-33.3
#[test]
fn query_kind_filters() -> TestResult {
    let (_dir, ws) = workspace()?;
    assert_eq!(
        hit_ids(&codex_view::query(&ws, ":clue", 20)?),
        ["C-031", "C-032"]
    );
    assert_eq!(
        hit_ids(&codex_view::query(&ws, ":clue murmur", 20)?),
        ["C-031"]
    );
    assert_eq!(
        hit_ids(&codex_view::query(&ws, ":Floor", 20)?),
        ["f04-choir-hall"]
    );
    let none = codex_view::query(&ws, ":nokind", 20)?;
    assert!(none.hits.is_empty());
    assert_eq!(none.note.as_deref(), Some("no kind `nokind`"));
    Ok(())
}

// AC: CE-33.3
#[test]
fn query_people_prefix_finds_the_npc() -> TestResult {
    let (_dir, ws) = workspace()?;
    let view = codex_view::query(&ws, "@Ilse", 20)?;
    assert_eq!(hit_ids(&view), ["mother-ilse"]);
    // The prefix keeps out records of other kinds that match the word.
    let shrine = codex_view::query(&ws, "@shrine", 20)?;
    assert!(
        shrine
            .hits
            .iter()
            .all(|h| matches!(h.record.kind.as_deref(), Some("people" | "faction" | "npc"))),
        "{:?}",
        hit_ids(&shrine)
    );
    Ok(())
}

#[test]
fn query_odd_input_is_no_error() -> TestResult {
    let (_dir, ws) = workspace()?;
    for text in ["#", ":", "@", "AND * -", "\"quoted phrase\"", "#zzz", "   "] {
        codex_view::query(&ws, text, 20).map_err(|e| format!("{text}: {e}"))?;
    }
    assert!(codex_view::query(&ws, "", 20)?.hits.is_empty());
    assert!(codex_view::query(&ws, "#zzz", 20)?.hits.is_empty());
    Ok(())
}

#[test]
fn query_truncates_at_the_limit() -> TestResult {
    let (_dir, ws) = workspace()?;
    let view = codex_view::query(&ws, ":location", 1)?;
    assert_eq!(view.hits.len(), 1);
    assert!(view.truncated);
    let word = codex_view::query(&ws, "synthetic", 1)?;
    assert_eq!(word.hits.len(), 1);
    assert!(word.truncated);
    Ok(())
}

// AC: CE-33.3
#[test]
fn a_plain_word_ranks_a_title_match_first() -> TestResult {
    let (_dir, ws) = workspace()?;
    // "climb" is in the title of `B-016` and in the voice lines of the floor's and `S-004`'s
    // bodies; no id holds it.
    let view = codex_view::query(&ws, "climb", 20)?;
    let ids: Vec<&str> = view.hits.iter().map(|h| h.record.id.as_str()).collect();
    assert_eq!(ids.first(), Some(&"B-016"), "{ids:?}");
    assert!(
        ids.contains(&"f04-choir-hall") && ids.contains(&"S-004"),
        "the body matches are hits too: {ids:?}"
    );
    assert_eq!(view.hits[0].matches[0].column, "title");
    assert!(
        view.hits[1..]
            .iter()
            .all(|hit| hit.matches.iter().all(|m| m.column == "body")),
        "the others match in the body only"
    );
    Ok(())
}
