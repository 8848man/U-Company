//! ADR-0049: reference material (long text, images) kept as files in `work-references/` instead of
//! inside the goal sentence. The app alone creates, names, splits and indexes these files from fixed
//! rules; an AI may only propose labels and section boundaries (the curator), which the app checks.
//! The SQLite `reference` records are the source of truth; INDEX.md / index.json are regenerated views.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Goal, Project};
use crate::storage::Store;

pub const FOLDER: &str = "work-references";
/// Pasted text above this many characters becomes a reference file (the composer applies it too).
pub const INLINE_MAX: usize = 4_000;
/// Text above this many characters is split into section files.
pub const SPLIT_OVER: usize = 20_000;
/// Target maximum section length; a table, code block or sentence is never cut, so a section may exceed it slightly.
pub const SECTION_MAX: usize = 8_000;
/// Sections shorter than this are merged into a neighbour.
pub const SECTION_MIN: usize = 1_000;
pub const TEXT_MAX_BYTES: usize = 1_048_576;
/// A formatted image (WebP, at most 2,576 px on the long edge) larger than this is refused.
pub const IMAGE_MAX_BYTES: usize = 4 * 1_048_576;
pub const IMAGE_MAX_EDGE: u32 = 2_576;
pub const GOAL_MAX_IMAGES: usize = 20;
pub const GOAL_MAX_BYTES: u64 = 50 * 1_048_576;
pub const SUMMARY_CHARS: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub n: usize,
    pub title: String,
    /// Project-relative path of the section file.
    pub file: String,
    pub chars: usize,
    pub first_line: String,
    #[serde(default)]
    pub summary: String,
    /// In requirement material: a section the curator marked as background (with its reason), which
    /// the PM may skip. Every other section of requirement material must be covered by the plan.
    #[serde(default)]
    pub background: bool,
    #[serde(default)]
    pub background_reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    /// `illustrates` (an image shows a text) or `relates_to` (same topic).
    pub kind: String,
    pub target: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: String,
    pub project_id: String,
    /// None while staged in the composer; set when the goal is submitted.
    #[serde(default)]
    pub goal_id: Option<String>,
    /// `R1`, `R2`, … within the goal; empty while staged.
    #[serde(default)]
    pub label: String,
    /// `text` or `image`.
    pub kind: String,
    /// `requirement` (the PM reads it and the plan must cover it) or `reference` (read on demand).
    #[serde(default)]
    pub role: String,
    /// `paste`, `file` or `image`.
    pub source: String,
    pub title: String,
    pub original_name: String,
    pub original_bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub chars: usize,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    pub bytes: u64,
    /// Project-relative path (a file, or the section folder of split text); empty while staged.
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub sections: Vec<Section>,
    #[serde(default)]
    pub summary: String,
    /// `rule`, `curator`, `cto` or `ceo`: who wrote the summary line.
    #[serde(default)]
    pub summary_source: String,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    /// `none`, `pending`, `done`, `failed` or `skipped`.
    #[serde(default)]
    pub curation: String,
    #[serde(default)]
    pub curation_note: String,
    /// Images kept at 2,576 px for dense screens.
    #[serde(default)]
    pub detail: bool,
    pub created_at: String,
    /// `staged`, `ready` or `missing`.
    pub status: String,
}

impl Reference {
    /// What an index line shows: the CEO caption, else the summary.
    pub fn line(&self) -> String {
        let text = if !self.caption.trim().is_empty() { self.caption.trim() } else { self.summary.trim() };
        one_line(text, SUMMARY_CHARS)
    }
}

pub fn one_line(text: &str, max: usize) -> String {
    let flat: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out: String = flat.chars().take(max).collect();
    if flat.chars().count() > max { out.push('…'); }
    out
}

// ----------------------------- static splitting -----------------------------

/// One indivisible piece of text: a paragraph, a heading line with its paragraph, a code block or a table.
#[derive(Clone, Debug, PartialEq)]
pub struct Atom {
    pub text: String,
    pub heading: Option<(usize, String)>,
}

fn heading_of(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_end();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
        let title = trimmed[hashes..].trim().trim_end_matches('#').trim();
        if !title.is_empty() { return Some((hashes, title.to_string())); }
    }
    None
}

fn is_fence(line: &str) -> bool { let t = line.trim_start(); t.starts_with("```") || t.starts_with("~~~") }

/// Splits text into atoms; their concatenation is exactly the input (CRLF normalized to LF first).
/// Fenced code is one atom and `#` inside it is never a heading; a setext heading (`Title` over `===`/`---`) is a heading.
pub fn atoms(text: &str) -> Vec<Atom> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut out: Vec<Atom> = Vec::new();
    let mut current = String::new();
    let mut heading: Option<(usize, String)> = None;
    let mut in_fence = false;
    let mut i = 0;
    let flush = |current: &mut String, heading: &mut Option<(usize, String)>, out: &mut Vec<Atom>| {
        if !current.is_empty() { out.push(Atom { text: std::mem::take(current), heading: heading.take() }); }
    };
    while i < lines.len() {
        let line = lines[i];
        let bare = line.trim_end_matches('\n');
        if in_fence {
            current.push_str(line);
            if is_fence(bare) { in_fence = false; }
            i += 1;
            continue;
        }
        if is_fence(bare) {
            current.push_str(line);
            in_fence = true;
            i += 1;
            continue;
        }
        if let Some(found) = heading_of(bare) {
            flush(&mut current, &mut heading, &mut out);
            heading = Some(found);
            current.push_str(line);
            i += 1;
            continue;
        }
        // Setext heading: a single non-blank line followed by === or ---.
        if current.is_empty() && !bare.trim().is_empty() && i + 1 < lines.len() {
            let next = lines[i + 1].trim();
            if !next.is_empty() && (next.chars().all(|c| c == '=') || (next.chars().all(|c| c == '-') && next.len() >= 3)) {
                heading = Some((if next.starts_with('=') { 1 } else { 2 }, bare.trim().to_string()));
                current.push_str(line);
                current.push_str(lines[i + 1]);
                i += 2;
                continue;
            }
        }
        current.push_str(line);
        if bare.trim().is_empty() {
            // A blank line closes the paragraph; the blank line stays with it so nothing is lost.
            flush(&mut current, &mut heading, &mut out);
        }
        i += 1;
    }
    flush(&mut current, &mut heading, &mut out);
    // A paragraph longer than a section is cut at line ends, then sentence ends, then characters.
    let mut sized = Vec::new();
    for atom in out {
        if atom.text.chars().count() <= SECTION_MAX || atom.text.trim_start().starts_with("```") || atom.text.trim_start().starts_with("~~~") || atom.text.trim_start().starts_with('|') {
            sized.push(atom);
            continue;
        }
        let mut first = true;
        for piece in cut_long(&atom.text) {
            sized.push(Atom { text: piece, heading: if first { atom.heading.clone() } else { None } });
            first = false;
        }
    }
    sized
}

fn cut_long(text: &str) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut count = 0;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        current.push(c);
        count += 1;
        let boundary = c == '\n' || ((c == '.' || c == '?' || c == '!' || c == '。') && chars.get(i + 1).is_some_and(|next| next.is_whitespace()));
        if count >= SECTION_MAX * 3 / 4 && boundary {
            if c != '\n' { if let Some(next) = chars.get(i + 1) { current.push(*next); i += 1; } }
            pieces.push(std::mem::take(&mut current));
            count = 0;
        } else if count >= SECTION_MAX * 3 / 2 {
            pieces.push(std::mem::take(&mut current));
            count = 0;
        }
        i += 1;
    }
    if !current.is_empty() { pieces.push(current); }
    pieces
}

/// A section before it is written: its title and text.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub title: String,
    pub text: String,
}

/// A section ready to write: its text plus what the curator said about it (if anything).
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub part: Part,
    pub summary: String,
    pub background: bool,
    pub reason: String,
}

impl Planned {
    pub fn plain(part: Part) -> Self { Self { part, summary: String::new(), background: false, reason: String::new() } }
}

/// What the curator decided per section, kept beside a staged record until the goal is sent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CuratedSection {
    pub title: String,
    pub start: usize,
    pub summary: String,
    #[serde(default)]
    pub background: bool,
    #[serde(default)]
    pub reason: String,
}

fn first_words(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("");
    one_line(line.trim_start_matches('#').trim(), 40)
}

fn chars(text: &str) -> usize { text.chars().count() }

/// Rule-based sections: cut at headings of level 1–2 (or 3 when there are none), split an oversized
/// group at deeper headings and then at paragraphs, and merge groups under `SECTION_MIN` into a neighbour.
/// The parts concatenate back to the input exactly.
pub fn split_static(text: &str) -> Vec<Part> {
    let atoms = atoms(text);
    let levels: BTreeSet<usize> = atoms.iter().filter_map(|atom| atom.heading.as_ref().map(|(level, _)| *level)).collect();
    let split_level = if levels.iter().any(|level| *level <= 2) { 2 } else if levels.contains(&3) { 3 } else { 0 };
    // Group atoms at split headings.
    let mut groups: Vec<(String, Vec<Atom>)> = Vec::new();
    for atom in atoms {
        let starts = split_level > 0 && atom.heading.as_ref().is_some_and(|(level, _)| *level <= split_level);
        if starts || groups.is_empty() {
            let title = match &atom.heading { Some((_, title)) => title.clone(), None => if split_level > 0 { "머리말".into() } else { first_words(&atom.text) } };
            groups.push((title, vec![atom]));
        } else {
            groups.last_mut().expect("group").1.push(atom);
        }
    }
    // Oversized groups: deeper headings first, then a greedy cut by paragraphs.
    let mut parts: Vec<Part> = Vec::new();
    for (title, atoms) in groups {
        let total: usize = atoms.iter().map(|atom| chars(&atom.text)).sum();
        if total <= SECTION_MAX {
            parts.push(Part { title, text: atoms.iter().map(|atom| atom.text.as_str()).collect() });
            continue;
        }
        let mut piece_title = title.clone();
        let mut piece = String::new();
        for (index, atom) in atoms.iter().enumerate() {
            let deeper = index > 0 && atom.heading.is_some();
            let would_overflow = chars(&piece) + chars(&atom.text) > SECTION_MAX;
            if !piece.is_empty() && (would_overflow || (deeper && chars(&piece) >= SECTION_MIN)) {
                parts.push(Part { title: piece_title.clone(), text: std::mem::take(&mut piece) });
                piece_title = match &atom.heading { Some((_, sub)) if split_level > 0 => format!("{title} > {sub}"), Some((_, sub)) => sub.clone(), None => if split_level > 0 { format!("{title} (이어서)") } else { first_words(&atom.text) } };
            }
            piece.push_str(&atom.text);
        }
        if !piece.is_empty() { parts.push(Part { title: piece_title, text: piece }); }
    }
    merge_small(parts)
}

fn merge_small(parts: Vec<Part>) -> Vec<Part> {
    let mut merged: Vec<Part> = Vec::new();
    let mut carry: Option<Part> = None;
    for part in parts {
        let part = match carry.take() {
            // A short preamble takes the next section's title; otherwise the earlier title leads.
            Some(small) if chars(&small.text) + chars(&part.text) <= SECTION_MAX => Part { title: if small.title == "머리말" { part.title.clone() } else { small.title }, text: small.text + &part.text },
            Some(small) => { merged.push(small); part }
            None => part,
        };
        if chars(&part.text) < SECTION_MIN { carry = Some(part); } else { merged.push(part); }
    }
    if let Some(small) = carry {
        match merged.last_mut() {
            Some(last) if chars(&last.text) + chars(&small.text) <= SECTION_MAX => last.text.push_str(&small.text),
            _ => merged.push(small),
        }
    }
    merged
}

/// Whether the text has its own structure (headings); unstructured long text is worth curating.
pub fn has_headings(text: &str) -> bool { atoms(text).iter().any(|atom| atom.heading.is_some()) }

/// First sentence (rule summary) for the index when nobody wrote one.
pub fn rule_summary(text: &str) -> String {
    let line = text.lines().map(str::trim).find(|line| !line.is_empty() && !line.starts_with("```")).unwrap_or("");
    one_line(line.trim_start_matches('#').trim(), SUMMARY_CHARS)
}

// ----------------------------- curator proposals -----------------------------

/// What the curator may propose. It never writes text: section starts are atom numbers (1-based)
/// the app showed it, so the sections still concatenate to the original.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Proposal {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub sections: Vec<ProposedSection>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProposedSection {
    pub start: usize,
    pub title: String,
    pub summary: String,
    /// Requirement material only: the curator is sure this section is background, not a requirement.
    pub background: bool,
    pub reason: String,
}

/// The atoms as the curator sees them: `[P3]` before each paragraph.
pub fn numbered(atoms: &[Atom]) -> String {
    atoms.iter().enumerate().map(|(index, atom)| format!("[P{}]\n{}", index + 1, atom.text.trim_end())).collect::<Vec<_>>().join("\n\n")
}

/// Checks a proposal against the atoms and other references. Sections are applied only if every
/// atom is covered once, in order, and no section is far beyond the limit; labels are cleaned.
pub fn check_proposal(proposal: &Proposal, atoms: &[Atom], self_id: &str, known_ids: &[String]) -> (Option<Vec<Planned>>, Vec<String>) {
    let mut notes = Vec::new();
    let mut sections = None;
    if !proposal.sections.is_empty() {
        let starts: Vec<usize> = proposal.sections.iter().map(|item| item.start).collect();
        let ordered = starts.first() == Some(&1) && starts.windows(2).all(|pair| pair[0] < pair[1]) && starts.last().is_some_and(|last| *last <= atoms.len());
        if !ordered {
            notes.push("정리안의 절 경계가 순서에 맞지 않아 규칙으로 나눴어요".into());
        } else {
            let mut built = Vec::new();
            for (index, item) in proposal.sections.iter().enumerate() {
                let end = proposal.sections.get(index + 1).map(|next| next.start - 1).unwrap_or(atoms.len());
                let text: String = atoms[item.start - 1..end].iter().map(|atom| atom.text.as_str()).collect();
                let title = if item.title.trim().is_empty() { first_words(&text) } else { one_line(&item.title, 80) };
                // A background mark without a reason is not trusted: the section stays a requirement.
                let background = item.background && !item.reason.trim().is_empty();
                built.push(Planned { part: Part { title, text }, summary: one_line(&item.summary, SUMMARY_CHARS), background, reason: if background { one_line(&item.reason, 160) } else { String::new() } });
            }
            if built.iter().any(|planned| chars(&planned.part.text) > SECTION_MAX * 3 / 2) {
                notes.push("정리안의 절 하나가 너무 길어 규칙으로 나눴어요".into());
            } else {
                sections = Some(built);
            }
        }
    }
    for relation in &proposal.relations {
        if !["illustrates", "relates_to"].contains(&relation.kind.as_str()) || relation.target == self_id || !known_ids.contains(&relation.target) {
            notes.push(format!("확인할 수 없는 관계({} → {})는 버렸어요", relation.kind, relation.target));
        }
    }
    (sections, notes)
}

pub fn clean_relations(relations: &[Relation], self_id: &str, known_ids: &[String]) -> Vec<Relation> {
    relations.iter().filter(|relation| ["illustrates", "relates_to"].contains(&relation.kind.as_str()) && relation.target != self_id && known_ids.contains(&relation.target))
        .map(|relation| Relation { kind: relation.kind.clone(), target: relation.target.clone(), note: one_line(&relation.note, 120) }).take(8).collect()
}

pub fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = one_line(tag, 20).trim_end_matches('…').to_string();
        if !tag.is_empty() && !out.contains(&tag) { out.push(tag); }
        if out.len() == 8 { break; }
    }
    out
}

/// The curator's answer: JSON, possibly inside a code fence or after a sentence.
pub fn parse_proposal(text: &str) -> Result<Proposal, String> {
    let start = text.find('{').ok_or("정리안 JSON을 찾지 못했습니다")?;
    let end = text.rfind('}').ok_or("정리안 JSON을 찾지 못했습니다")?;
    if end < start { return Err("정리안 JSON을 찾지 못했습니다".into()); }
    serde_json::from_str(&text[start..=end]).map_err(|e| format!("정리안 형식이 맞지 않습니다: {e}"))
}

// ----------------------------- names and paths -----------------------------

/// ASCII-only, lowercase, short: never two names that differ only by case on Windows.
pub fn slug(name: &str) -> String {
    let stem = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let mut out = String::new();
    for c in stem.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() { out.push(c); } else if !out.ends_with('-') && !out.is_empty() { out.push('-'); }
        if out.len() >= 32 { break; }
    }
    out.trim_end_matches('-').to_string()
}

pub fn goal_folder(goal: &Goal) -> String {
    let date: String = goal.created_at.chars().take(10).collect();
    let short: String = goal.id.chars().filter(|c| c.is_ascii_alphanumeric()).take(6).collect::<String>().to_ascii_lowercase();
    format!("{FOLDER}/{date}_{short}")
}

pub fn is_reference_path(path: &str) -> bool { path.split('/').next().is_some_and(|first| first.eq_ignore_ascii_case(FOLDER)) }

fn staging_dir(staging_root: &Path, id: &str) -> PathBuf { staging_root.join(id) }

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

// ----------------------------- staging (composer) -----------------------------

/// Text from a paste or an attached `.md`/`.txt`, held in app data until the goal is sent.
pub fn stage_text(store: &Store, staging_root: &Path, project: &Project, name: &str, text: &str, source: &str) -> Result<Reference, String> {
    if text.len() > TEXT_MAX_BYTES { return Err("글 자료는 1MB까지 첨부할 수 있어요".into()); }
    let text = text.replace("\r\n", "\n").trim_start_matches('\u{feff}').to_string();
    if text.trim().is_empty() { return Err("빈 글은 첨부할 수 없어요".into()); }
    if !["paste", "file"].contains(&source) { return Err("알 수 없는 자료 종류입니다".into()); }
    let id = crate::model::id();
    let dir = staging_dir(staging_root, &id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("content.md"), &text).map_err(|e| e.to_string())?;
    let original_name = if source == "paste" { "붙여넣은 글".to_string() } else { one_line(name, 120) };
    let title = if source == "paste" { first_words(&text) } else { original_name.clone() };
    let role = suggest_role(&original_name, &text);
    let curate = curation_wanted(store, role, &text)?;
    let reference = Reference {
        id: id.clone(), project_id: project.id.clone(), goal_id: None, label: String::new(), kind: "text".into(), role: role.into(), source: source.into(),
        title, original_name, original_bytes: text.len() as u64, sha256: sha256_hex(text.as_bytes()), chars: chars(&text), width: 0, height: 0,
        bytes: text.len() as u64, path: String::new(), sections: vec![], summary: rule_summary(&text), summary_source: "rule".into(), caption: String::new(),
        tags: vec![], relations: vec![], curation: if curate { "pending".into() } else { "none".into() }, curation_note: String::new(),
        detail: false, created_at: crate::model::now(), status: "staged".into(),
    };
    store.put("reference", &id, &reference, None)?;
    Ok(reference)
}

/// ADR-0049 (revised): the curator is off unless the CEO turns it on. When on, it runs only for split
/// text (over `SPLIT_OVER`) that has no headings (topic grouping) or is requirement material
/// (marking background sections the PM may skip). Images are not curated.
pub fn needs_curation_text(role: &str, text: &str) -> bool { chars(text) > SPLIT_OVER && (role == "requirement" || !has_headings(text)) }

pub fn curator_enabled(store: &Store) -> Result<bool, String> { Ok(store.get::<bool>("setting", "curator_enabled")?.unwrap_or(false)) }

pub fn set_curator_enabled(store: &Store, enabled: bool) -> Result<(), String> { store.put("setting", "curator_enabled", &enabled, None) }

fn curation_wanted(store: &Store, role: &str, text: &str) -> Result<bool, String> { Ok(curator_enabled(store)? && needs_curation_text(role, text)) }

/// A name or first heading that says "requirements" makes the material a requirement by default;
/// the CEO can change it on the chip.
pub fn suggest_role(name: &str, text: &str) -> &'static str {
    let heading = text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or("");
    let probe = format!("{name} {heading}").to_lowercase();
    if ["요구사항", "요건", "명세", "기획서", "spec", "prd", "requirement"].iter().any(|word| probe.contains(word)) { "requirement" } else { "reference" }
}

/// The CEO's choice on the chip. Returns the record and whether curation should start now.
pub fn set_role(store: &Store, staging_root: &Path, id: &str, role: &str) -> Result<(Reference, bool), String> {
    if !["requirement", "reference"].contains(&role) { return Err("자료 역할은 요구사항 또는 참고여야 합니다".into()); }
    let mut reference: Reference = store.get("reference", id)?.ok_or("자료를 찾을 수 없습니다")?;
    if reference.kind != "text" { return Err("글 자료만 요구사항으로 지정할 수 있어요".into()); }
    reference.role = role.into();
    let mut start = false;
    if reference.status == "staged" && reference.curation == "none" {
        if let Ok(text) = staged_text(staging_root, id) { if curation_wanted(store, role, &text)? { reference.curation = "pending".into(); start = true; } }
    }
    store.put("reference", id, &reference, None)?;
    Ok((reference, start))
}

/// One requirement the plan must cover: `R2 §3` (or `R2` for unsplit text) and the file that holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct RequirementTarget {
    pub key: String,
    pub file: String,
}

/// Requirement material of a goal, section by section, minus the sections marked as background.
pub fn requirement_targets(refs: &[Reference]) -> Vec<RequirementTarget> {
    let mut targets = Vec::new();
    for reference in refs.iter().filter(|item| item.role == "requirement" && item.kind == "text") {
        if reference.sections.is_empty() { targets.push(RequirementTarget { key: reference.label.clone(), file: reference.path.clone() }); continue; }
        for section in reference.sections.iter().filter(|section| !section.background) {
            targets.push(RequirementTarget { key: format!("{} §{}", reference.label, section.n), file: section.file.clone() });
        }
    }
    targets
}

/// Every requirement must be some task's evidence, or be excluded by the plan with a reason.
pub fn check_coverage(targets: &[RequirementTarget], evidence: &[&str], exclusions: &[(String, String)]) -> Result<(), String> {
    let normalized = |key: &str| key.split_whitespace().collect::<Vec<_>>().join(" ").replace("§ ", "§").to_ascii_uppercase();
    let missing: Vec<&str> = targets.iter().filter(|target| {
        let used = evidence.iter().any(|path| path.eq_ignore_ascii_case(&target.file));
        let excluded = exclusions.iter().any(|(key, reason)| !reason.trim().is_empty() && (normalized(key) == normalized(&target.key) || normalized(key) == normalized(target.key.split(' ').next().unwrap_or(""))));
        !used && !excluded
    }).map(|target| target.key.as_str()).collect();
    if missing.is_empty() { return Ok(()); }
    Err(format!("요구사항 {}이(가) 어떤 업무의 근거 자료에도 없습니다. 해당 절 파일을 업무의 evidence에 넣거나, 반영하지 않을 이유를 requirementExclusions에 적어 주세요", missing.join(", ")))
}

pub struct ImageMeta {
    pub original_name: String,
    pub original_bytes: u64,
    pub original_sha256: String,
    pub caption: String,
    pub detail: bool,
}

/// An image already formatted by the screen (WebP, metadata dropped). The backend checks the file
/// signature and the size again; it never trusts the screen's numbers alone.
pub fn stage_image(store: &Store, staging_root: &Path, project: &Project, bytes: &[u8], meta: ImageMeta) -> Result<Reference, String> {
    if bytes.len() > IMAGE_MAX_BYTES { return Err("변환한 이미지가 4MB를 넘어요. 더 작은 이미지를 첨부해 주세요".into()); }
    let (width, height) = webp_size(bytes).ok_or("WebP 이미지가 아닙니다. 앱 화면에서 다시 첨부해 주세요")?;
    if width == 0 || height == 0 || width.max(height) > IMAGE_MAX_EDGE { return Err(format!("이미지 긴 변은 {IMAGE_MAX_EDGE}px 이하여야 해요")); }
    if !meta.original_sha256.chars().all(|c| c.is_ascii_hexdigit()) || meta.original_sha256.len() != 64 { return Err("원본 이미지 확인값이 올바르지 않습니다".into()); }
    let id = crate::model::id();
    let dir = staging_dir(staging_root, &id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("image.webp"), bytes).map_err(|e| e.to_string())?;
    let original_name = one_line(&meta.original_name, 120);
    let reference = Reference {
        id: id.clone(), project_id: project.id.clone(), goal_id: None, label: String::new(), kind: "image".into(), role: "reference".into(), source: "image".into(),
        title: original_name.clone(), original_name, original_bytes: meta.original_bytes, sha256: meta.original_sha256.to_ascii_lowercase(), chars: 0, width, height,
        bytes: bytes.len() as u64, path: String::new(), sections: vec![], summary: String::new(), summary_source: if meta.caption.trim().is_empty() { "rule".into() } else { "ceo".into() },
        caption: one_line(&meta.caption, SUMMARY_CHARS), tags: vec![], relations: vec![], curation: "none".into(), curation_note: String::new(),
        detail: meta.detail, created_at: crate::model::now(), status: "staged".into(),
    };
    store.put("reference", &id, &reference, None)?;
    Ok(reference)
}

/// Width and height from a WebP header (VP8, VP8L or VP8X); None when the bytes are not WebP.
pub fn webp_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 30 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" { return None; }
    let le24 = |b: &[u8]| (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16);
    match &bytes[12..16] {
        b"VP8 " => {
            if bytes[23..26] != [0x9d, 0x01, 0x2a] { return None; }
            let w = u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3fff;
            let h = u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3fff;
            Some((w as u32, h as u32))
        }
        b"VP8L" => {
            if bytes[20] != 0x2f { return None; }
            let b = &bytes[21..25];
            let w = 1 + (((b[1] as u32 & 0x3f) << 8) | b[0] as u32);
            let h = 1 + (((b[3] as u32 & 0x0f) << 10) | ((b[2] as u32) << 2) | ((b[1] as u32 & 0xc0) >> 6));
            Some((w, h))
        }
        b"VP8X" => Some((1 + le24(&bytes[24..27]), 1 + le24(&bytes[27..30]))),
        _ => None,
    }
}

/// Standard base64 (the screen sends formatted image bytes this way).
pub fn decode_base64(text: &str) -> Result<Vec<u8>, String> {
    let value = |c: u8| -> Option<u32> { match c { b'A'..=b'Z' => Some((c - b'A') as u32), b'a'..=b'z' => Some((c - b'a' + 26) as u32), b'0'..=b'9' => Some((c - b'0' + 52) as u32), b'+' => Some(62), b'/' => Some(63), _ => None } };
    let clean: Vec<u8> = text.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let body = clean.strip_suffix(b"==").or_else(|| clean.strip_suffix(b"=")).unwrap_or(&clean);
    let mut out = Vec::with_capacity(body.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0;
    for byte in body {
        buffer = (buffer << 6) | value(*byte).ok_or("이미지 데이터가 올바르지 않습니다")?;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((buffer >> bits) as u8); buffer &= (1 << bits) - 1; }
    }
    Ok(out)
}

/// Staged material the CEO never sent is removed after a week.
pub fn cleanup_stale(store: &Store, staging_root: &Path) -> Result<usize, String> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
    let mut removed = 0;
    for reference in store.list::<Reference>("reference")?.into_iter().filter(|item| item.status == "staged" && item.created_at < cutoff) {
        let _ = std::fs::remove_dir_all(staging_dir(staging_root, &reference.id));
        store.transact(|tx| { tx.execute("DELETE FROM objects WHERE kind IN ('reference','reference_curated_sections') AND id=?1", [reference.id.as_str()]).map_err(|e| e.to_string())?; Ok(()) })?;
        removed += 1;
    }
    Ok(removed)
}

pub fn staged_text(staging_root: &Path, id: &str) -> Result<String, String> {
    std::fs::read_to_string(staging_dir(staging_root, id).join("content.md")).map_err(|e| format!("첨부한 글을 찾을 수 없습니다: {e}"))
}

pub fn staged_image_path(staging_root: &Path, id: &str) -> PathBuf { staging_dir(staging_root, id).join("image.webp") }

/// Removes a staged reference (the chip's ×). Materialized references are not removed here.
pub fn discard_staged(store: &Store, staging_root: &Path, id: &str) -> Result<(), String> {
    let reference: Reference = store.get("reference", id)?.ok_or("자료를 찾을 수 없습니다")?;
    if reference.status != "staged" { return Err("목표에 이미 들어간 자료는 여기서 지울 수 없습니다".into()); }
    let _ = std::fs::remove_dir_all(staging_dir(staging_root, id));
    store.transact(|tx| { tx.execute("DELETE FROM objects WHERE kind='reference' AND id=?1", [id]).map_err(|e| e.to_string())?; Ok(()) })
}

/// The CEO's caption or "insert into the goal text instead" never touch files; only the caption is stored.
pub fn set_caption(store: &Store, id: &str, caption: &str) -> Result<Reference, String> {
    let mut reference: Reference = store.get("reference", id)?.ok_or("자료를 찾을 수 없습니다")?;
    reference.caption = one_line(caption, SUMMARY_CHARS);
    if !reference.caption.is_empty() { reference.summary_source = if reference.summary_source == "rule" { "ceo".into() } else { reference.summary_source }; }
    store.put("reference", id, &reference, None)?;
    Ok(reference)
}

// ----------------------------- materializing into the goal folder -----------------------------

/// Moves staged references into `work-references/<goal folder>/`, numbers them R1…, splits long text,
/// and regenerates both indexes. Called by `submit_goal`; all checks run before any file is written.
pub fn attach_to_goal(store: &Store, staging_root: &Path, project: &Project, goal: &Goal, ids: &[String]) -> Result<Vec<Reference>, String> {
    if ids.is_empty() { return Ok(vec![]); }
    let mut staged = Vec::new();
    for id in ids {
        let reference: Reference = store.get("reference", id)?.ok_or("첨부한 자료가 사라졌습니다. 다시 첨부해 주세요")?;
        if reference.status != "staged" || reference.project_id != project.id { return Err("이 프로젝트에 첨부한 자료가 아닙니다".into()); }
        staged.push(reference);
    }
    let images = staged.iter().filter(|item| item.kind == "image").count();
    if images > GOAL_MAX_IMAGES { return Err(format!("이미지는 목표당 {GOAL_MAX_IMAGES}장까지 첨부할 수 있어요")); }
    if staged.iter().map(|item| item.bytes).sum::<u64>() > GOAL_MAX_BYTES { return Err("자료는 목표당 50MB까지 첨부할 수 있어요".into()); }
    let root = Path::new(&project.path);
    let folder = goal_folder(goal);
    let mut attached = Vec::new();
    for (index, mut reference) in staged.into_iter().enumerate() {
        let label = format!("R{}", index + 1);
        let name = slug(&reference.original_name);
        let base = if name.is_empty() { label.clone() } else { format!("{label}-{name}") };
        if reference.kind == "image" {
            let relative = format!("{folder}/images/{base}.webp");
            write_file(root, &relative, &std::fs::read(staged_image_path(staging_root, &reference.id)).map_err(|e| e.to_string())?)?;
            reference.path = relative;
        } else {
            let text = staged_text(staging_root, &reference.id)?;
            if chars(&text) <= SPLIT_OVER {
                let relative = format!("{folder}/text/{base}.md");
                write_file(root, &relative, text.as_bytes())?;
                reference.path = relative;
            } else {
                let dir = format!("{folder}/text/{base}");
                let parts = staged_sections(store, &reference, &text);
                reference.sections = write_sections(root, &dir, &reference.original_name, &parts)?;
                reference.path = format!("{dir}/");
            }
        }
        reference.label = label;
        reference.goal_id = Some(goal.id.clone());
        reference.status = "ready".into();
        store.put("reference", &reference.id, &reference, None)?;
        let _ = std::fs::remove_dir_all(staging_dir(staging_root, &reference.id));
        attached.push(reference);
    }
    // Relations named staged ids; keep them, they are the same ids after attaching.
    exclude_from_git(root, project)?;
    write_indexes(store, project)?;
    Ok(attached)
}

/// Sections the curator proposed (kept beside the staged record), else the static rules.
fn staged_sections(store: &Store, reference: &Reference, text: &str) -> Vec<Planned> {
    let fallback = || split_static(text).into_iter().map(Planned::plain).collect();
    if let Ok(Some(saved)) = store.get::<Vec<CuratedSection>>("reference_curated_sections", &reference.id) {
        let atoms = atoms(text);
        let mut out = Vec::new();
        for (index, item) in saved.iter().enumerate() {
            let end = saved.get(index + 1).map(|next| next.start.saturating_sub(1)).unwrap_or(atoms.len());
            if item.start == 0 || item.start > end || end > atoms.len() { return fallback(); }
            // Background marks only mean something in requirement material.
            let background = reference.role == "requirement" && item.background;
            out.push(Planned { part: Part { title: item.title.clone(), text: atoms[item.start - 1..end].iter().map(|atom| atom.text.as_str()).collect() }, summary: item.summary.clone(), background, reason: if background { item.reason.clone() } else { String::new() } });
        }
        if !out.is_empty() { return out; }
    }
    fallback()
}

fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    if !is_reference_path(relative) || relative.contains("..") || relative.contains('\\') { return Err("참조 자료 경로가 올바르지 않습니다".into()); }
    let target = root.join(relative);
    if let Some(parent) = target.parent() { std::fs::create_dir_all(parent).map_err(|e| format!("참조 자료 폴더를 만들지 못했습니다: {e}"))?; }
    std::fs::write(&target, bytes).map_err(|e| format!("참조 자료를 저장하지 못했습니다: {e}"))
}

fn write_sections(root: &Path, dir: &str, original: &str, parts: &[Planned]) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    for (index, Planned { part, summary, background, reason }) in parts.iter().enumerate() {
        let n = index + 1;
        let file = format!("{dir}/{n:02}.md");
        // The first line tells a teammate who only got this part where it sits in the whole.
        write_file(root, &file, format!("> 출처: {} · §{n} {}\n\n{}", one_line(original, 80), part.title, part.text).as_bytes())?;
        sections.push(Section { n, title: part.title.clone(), file, chars: chars(&part.text), first_line: first_words(&part.text), summary: summary.clone(), background: *background, background_reason: reason.clone() });
    }
    let mut outline = format!("# 목차 · {}\n\n| § | 제목 | 글자 수 | 첫 문장 |\n| --- | --- | --- | --- |\n", one_line(original, 80));
    for section in &sections {
        outline.push_str(&format!("| {} | {} | {} | {} |\n", section.n, cell(&section.title), section.chars, cell(&section.first_line)));
    }
    write_file(root, &format!("{dir}/00-outline.md"), outline.as_bytes())?;
    Ok(sections)
}

fn cell(text: &str) -> String { one_line(text, 120).replace('|', "\\|") }

/// `work-references/` stays out of `git status` through the repository's local exclude file
/// (`.git/info/exclude`), which is never committed or shared.
pub fn exclude_from_git(root: &Path, project: &Project) -> Result<(), String> {
    if project.kind != "git" { return Ok(()); }
    let info = root.join(".git").join("info");
    if !root.join(".git").is_dir() { return Ok(()); }
    std::fs::create_dir_all(&info).map_err(|e| e.to_string())?;
    let path = info.join("exclude");
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let line = format!("/{FOLDER}/");
    if current.lines().any(|existing| existing.trim() == line) { return Ok(()); }
    let mut next = current;
    if !next.is_empty() && !next.ends_with('\n') { next.push('\n'); }
    next.push_str(&format!("# U-Company reference material (ADR-0049)\n{line}\n"));
    std::fs::write(&path, next).map_err(|e| e.to_string())
}

// ----------------------------- indexes -----------------------------

pub fn project_references(store: &Store, project_id: &str) -> Result<Vec<Reference>, String> {
    Ok(store.list::<Reference>("reference")?.into_iter().filter(|item| item.project_id == project_id && item.goal_id.is_some()).collect())
}

/// References a goal may use: its own and those inherited from earlier versions (direction changes).
pub fn for_goal(store: &Store, goal_id: &str) -> Result<Vec<Reference>, String> {
    let chain = lineage(store, goal_id)?;
    let mut refs: Vec<Reference> = store.list::<Reference>("reference")?.into_iter().filter(|item| item.goal_id.as_ref().is_some_and(|id| chain.contains(id))).collect();
    refs.sort_by(|a, b| (a.goal_id.as_deref() != Some(goal_id)).cmp(&(b.goal_id.as_deref() != Some(goal_id))).then(label_number(&a.label).cmp(&label_number(&b.label))));
    Ok(refs)
}

fn label_number(label: &str) -> usize { label.trim_start_matches('R').parse().unwrap_or(usize::MAX) }

/// This goal and every earlier version it replaced (newest first).
pub fn lineage(store: &Store, goal_id: &str) -> Result<Vec<String>, String> {
    let guidance = store.list::<crate::model::GoalGuidance>("goal_guidance")?;
    let mut chain = vec![goal_id.to_string()];
    let mut current = goal_id.to_string();
    while let Some(previous) = guidance.iter().find(|item| item.replacement_goal_id.as_deref() == Some(current.as_str())).map(|item| item.goal_id.clone()) {
        if chain.contains(&previous) { break; }
        chain.push(previous.clone());
        current = previous;
    }
    Ok(chain)
}

/// The goal index as text: what CTO and PM read instead of the material itself.
pub fn goal_index_md(goal: &Goal, refs: &[Reference]) -> String {
    let mut out = format!("# 참조 자료 · {} ({})\n\n", one_line(&goal.intent, 60), goal_folder(goal).trim_start_matches(&format!("{FOLDER}/")));
    if refs.is_empty() { out.push_str("자료 없음\n"); return out; }
    out.push_str("| ID | 종류 | 경로 | 크기 | 요약 | 태그 | 관계 |\n| --- | --- | --- | --- | --- | --- | --- |\n");
    for reference in refs {
        let base = if reference.role == "requirement" { "요구사항" } else { "글" };
        let kind = if reference.kind == "image" { "이미지".to_string() } else if reference.sections.is_empty() { base.to_string() } else { format!("{base}({}절)", reference.sections.len()) };
        let size = if reference.kind == "image" { format!("{}×{}, {}KB", reference.width, reference.height, reference.bytes.div_ceil(1024)) } else { format!("{}자", reference.chars) };
        let inherited = if reference.goal_id.as_deref() != Some(goal.id.as_str()) { " (이전 버전)" } else { "" };
        let relations = reference.relations.iter().map(|relation| format!("{} {}", relation.kind, label_of(refs, &relation.target))).collect::<Vec<_>>().join(", ");
        out.push_str(&format!("| {}{} | {} | {} | {} | {} | {} | {} |\n", reference.label, inherited, kind, reference.path, size, cell(&reference.line()), cell(&reference.tags.join(", ")), cell(&relations)));
        for section in &reference.sections {
            let line = if section.summary.is_empty() { &section.first_line } else { &section.summary };
            let kind = if section.background { format!("배경 ({})", cell(&section.background_reason)) } else if reference.role == "requirement" { "요구사항 절".to_string() } else { "절".to_string() };
            out.push_str(&format!("| {} §{} | {} | {} | {}자 | {} — {} | | |\n", reference.label, section.n, kind, section.file, section.chars, cell(&section.title), cell(line)));
        }
    }
    out
}

/// One line a teammate gets beside an evidence path that is reference material.
pub fn evidence_note(refs: &[Reference], path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    for reference in refs {
        if reference.path.to_ascii_lowercase() == lower { return Some(format!("{} {}", reference.label, reference.line())); }
        if let Some(section) = reference.sections.iter().find(|section| section.file.to_ascii_lowercase() == lower) {
            let line = if section.summary.is_empty() { &section.first_line } else { &section.summary };
            return Some(format!("{} §{} {} — {}", reference.label, section.n, section.title, one_line(line, 120)));
        }
    }
    None
}

fn label_of(refs: &[Reference], id: &str) -> String { refs.iter().find(|item| item.id == id).map(|item| item.label.clone()).unwrap_or_else(|| "?".into()) }

/// Regenerates `work-references/INDEX.md`, `index.json` and each goal folder's pair from the records.
/// AI edits to these files are overwritten here; files the user deleted are marked `missing`.
pub fn write_indexes(store: &Store, project: &Project) -> Result<(), String> {
    let root = Path::new(&project.path);
    let refs = project_references(store, &project.id)?;
    if refs.is_empty() { return Ok(()); }
    let goals: Vec<Goal> = store.list::<Goal>("goal")?;
    let mut by_goal: Vec<(&Goal, Vec<Reference>)> = Vec::new();
    for goal in goals.iter().filter(|goal| refs.iter().any(|item| item.goal_id.as_deref() == Some(goal.id.as_str()))) {
        let mut own = Vec::new();
        for mut reference in refs.iter().filter(|item| item.goal_id.as_deref() == Some(goal.id.as_str())).cloned() {
            let exists = if reference.sections.is_empty() { root.join(&reference.path).is_file() } else { reference.sections.iter().all(|section| root.join(&section.file).is_file()) };
            let status = if exists { "ready" } else { "missing" };
            if reference.status != status { reference.status = status.into(); store.put("reference", &reference.id, &reference, None)?; }
            own.push(reference);
        }
        own.sort_by_key(|item| label_number(&item.label));
        by_goal.push((goal, own));
    }
    let mut root_md = String::from("# 참조 자료\n\n앱이 만드는 목록입니다. 직접 고쳐도 다음 갱신 때 다시 만들어집니다.\n\n| 목표 폴더 | 목표 | 자료 수 | 만든 날 |\n| --- | --- | --- | --- |\n");
    let mut root_json = Vec::new();
    for (goal, own) in &by_goal {
        let folder = goal_folder(goal);
        root_md.push_str(&format!("| {}/ | {} | {} | {} |\n", folder.trim_start_matches(&format!("{FOLDER}/")), cell(&one_line(&goal.intent, 60)), own.len(), goal.created_at.chars().take(10).collect::<String>()));
        root_json.push(serde_json::json!({ "goalId": goal.id, "folder": folder, "references": own.len() }));
        write_file(root, &format!("{folder}/INDEX.md"), goal_index_md(goal, own).as_bytes())?;
        let json = serde_json::json!({ "goalId": goal.id, "folder": folder, "references": own.iter().map(|item| serde_json::json!({
            "id": item.label, "kind": item.kind, "path": item.path, "title": item.title, "summary": item.line(), "summarySource": item.summary_source,
            "tags": item.tags, "chars": item.chars, "width": item.width, "height": item.height, "bytes": item.bytes, "status": item.status,
            "derivedFrom": { "name": item.original_name, "bytes": item.original_bytes, "sha256": item.sha256 },
            "sections": item.sections.iter().map(|section| serde_json::json!({ "n": section.n, "title": section.title, "file": section.file, "chars": section.chars, "summary": section.summary })).collect::<Vec<_>>(),
            "relations": item.relations.iter().map(|relation| serde_json::json!({ "kind": relation.kind, "target": label_of(own, &relation.target), "note": relation.note })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>() });
        write_file(root, &format!("{folder}/index.json"), serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?.as_bytes())?;
    }
    write_file(root, &format!("{FOLDER}/INDEX.md"), root_md.as_bytes())?;
    write_file(root, &format!("{FOLDER}/index.json"), serde_json::to_string_pretty(&serde_json::json!({ "goals": root_json })).map_err(|e| e.to_string())?.as_bytes())?;
    Ok(())
}

/// After a goal's backup is deleted: its folder goes, unless a later version still uses the
/// material, in which case the records move to that version and the files stay where they are.
/// The later version that replaced this goal, read before the goal's own records are deleted.
pub fn heir_of(store: &Store, goal_id: &str) -> Result<Option<String>, String> {
    Ok(store.list::<crate::model::GoalGuidance>("goal_guidance")?.into_iter().find(|item| item.goal_id == goal_id && item.replacement_goal_id.is_some()).and_then(|item| item.replacement_goal_id))
}

pub fn purge_goal(store: &Store, project: &Project, goal: &Goal, heir: Option<String>) -> Result<(), String> {
    let refs: Vec<Reference> = store.list::<Reference>("reference")?.into_iter().filter(|item| item.goal_id.as_deref() == Some(goal.id.as_str())).collect();
    if refs.is_empty() { return Ok(()); }
    let heir_alive = heir.as_ref().is_some_and(|id| store.get::<Goal>("goal", id).ok().flatten().is_some());
    let root = Path::new(&project.path);
    for mut reference in refs {
        if heir_alive {
            reference.goal_id = heir.clone();
            store.put("reference", &reference.id, &reference, None)?;
            continue;
        }
        // The files may sit in an earlier version's folder (inherited material): remove them where they are.
        if is_reference_path(&reference.path) && !reference.path.contains("..") {
            let target = root.join(reference.path.trim_end_matches('/'));
            if target.is_dir() { std::fs::remove_dir_all(&target).map_err(|e| format!("참조 자료를 지우지 못했습니다: {e}"))?; } else if target.is_file() { std::fs::remove_file(&target).map_err(|e| format!("참조 자료를 지우지 못했습니다: {e}"))?; }
        }
        store.transact(|tx| { tx.execute("DELETE FROM objects WHERE kind='reference' AND id=?1", [reference.id.as_str()]).map_err(|e| e.to_string())?; Ok(()) })?;
    }
    if !heir_alive {
        let dir = root.join(goal_folder(goal));
        if dir.is_dir() { std::fs::remove_dir_all(&dir).map_err(|e| format!("참조 자료 폴더를 지우지 못했습니다: {e}"))?; }
    }
    write_indexes(store, project)
}

/// Lines of `자료 요약 R1: …` / `자료 요약 R2 §3: …` in the CTO analysis fill summaries nobody wrote yet.
pub fn apply_cto_summaries(store: &Store, goal_id: &str, analysis: &str) -> Result<usize, String> {
    let mut refs = for_goal(store, goal_id)?;
    let mut changed = 0;
    for line in analysis.lines() {
        let Some(rest) = line.trim().trim_start_matches(['-', '*', ' ']).strip_prefix("자료 요약") else { continue };
        let Some((key, text)) = rest.split_once(':') else { continue };
        let key = key.trim();
        let (label, section) = match key.split_once('§') { Some((label, n)) => (label.trim(), n.trim().parse::<usize>().ok()), None => (key, None) };
        let Some(reference) = refs.iter_mut().find(|item| item.label == label && item.goal_id.as_deref() == Some(goal_id)) else { continue };
        match section {
            Some(n) => if let Some(section) = reference.sections.iter_mut().find(|item| item.n == n && item.summary.is_empty()) { section.summary = one_line(text, SUMMARY_CHARS); changed += 1; },
            None => if reference.summary_source == "rule" { reference.summary = one_line(text, SUMMARY_CHARS); reference.summary_source = "cto".into(); changed += 1; },
        }
    }
    for reference in refs.iter().filter(|item| item.goal_id.as_deref() == Some(goal_id)) { store.put("reference", &reference.id, reference, None)?; }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(parts: &[Part]) -> String { parts.iter().map(|part| part.text.as_str()).collect() }

    fn para(label: &str, n: usize) -> String { format!("{label} {}\n\n", "가나다라마바사아자차. ".repeat(n)) }

    #[test]
    fn headings_cut_sections_and_the_parts_rebuild_the_text_exactly() {
        let mut text = String::from("머리 문단입니다.\n\n");
        for title in ["개요", "가격", "경쟁사", "일정", "위험"] { text.push_str(&format!("## {title}\n\n")); for _ in 0..4 { text.push_str(&para(title, 30)); } }
        let parts = split_static(&text);
        assert_eq!(joined(&parts), text, "nothing is lost or reordered");
        assert!(parts.iter().any(|part| part.title == "가격"));
        assert!(parts.iter().all(|part| chars(&part.text) <= SECTION_MAX + 400), "{:?}", parts.iter().map(|p| chars(&p.text)).collect::<Vec<_>>());
        assert!(!parts.iter().any(|part| part.title == "머리말"), "a short preamble merges into the next section");
    }

    #[test]
    fn code_fences_hide_headings_and_are_never_cut() {
        let code = format!("```bash\n# not a heading\n{}```\n\n", "echo hi\n".repeat(1500));
        let text = format!("# 설치\n\n{}{code}# 사용\n\n{}", para("설치", 40), para("사용", 40));
        let atoms = atoms(&text);
        assert!(!atoms.iter().any(|atom| atom.heading.as_ref().is_some_and(|(_, title)| title == "not a heading")));
        assert!(atoms.iter().any(|atom| atom.text.starts_with("```bash") && atom.text.contains("echo hi\n```")), "the fence stays one atom even past the section limit");
        assert_eq!(joined(&split_static(&text)), text);
    }

    #[test]
    fn plain_paragraphs_group_up_to_the_limit_and_a_blob_cuts_at_sentence_ends() {
        let text: String = (0..30).map(|i| para(&format!("문단{i}"), 60)).collect();
        let parts = split_static(&text);
        assert_eq!(joined(&parts), text);
        assert!(parts.len() >= 3 && parts.iter().all(|part| chars(&part.text) <= SECTION_MAX));
        let blob = "한 줄로 이어진 긴 문장입니다. ".repeat(3000);
        let parts = split_static(&blob);
        assert_eq!(joined(&parts), blob);
        assert!(parts.len() > 3);
        assert!(parts.iter().take(parts.len() - 1).all(|part| part.text.ends_with(". ")), "cuts land after a sentence");
    }

    #[test]
    fn setext_headings_and_crlf_are_understood() {
        let text = "개요\n====\n\n본문\r\n\r\n가격\n----\n\n표\n";
        let atoms = atoms(text);
        assert_eq!(atoms.iter().filter_map(|atom| atom.heading.clone()).collect::<Vec<_>>(), vec![(1, "개요".to_string()), (2, "가격".to_string())]);
        assert_eq!(atoms.iter().map(|atom| atom.text.as_str()).collect::<String>(), text.replace("\r\n", "\n"));
    }

    #[test]
    fn a_curator_proposal_is_applied_only_when_it_covers_every_paragraph_in_order() {
        let text: String = (0..6).map(|i| para(&format!("P{i}"), 3)).collect();
        let atoms = atoms(&text);
        let good = Proposal { sections: vec![ProposedSection { start: 1, title: "가격".into(), summary: "가격 이야기".into(), ..Default::default() }, ProposedSection { start: 4, title: "일정\n".into(), summary: String::new(), ..Default::default() }], ..Default::default() };
        let (sections, notes) = check_proposal(&good, &atoms, "me", &[]);
        let sections = sections.expect("valid");
        assert_eq!(sections.iter().map(|planned| planned.part.text.as_str()).collect::<String>(), text);
        assert_eq!(sections[1].part.title, "일정");
        assert!(notes.is_empty());
        for bad in [vec![2, 4], vec![1, 1], vec![1, 9]] {
            let proposal = Proposal { sections: bad.iter().map(|start| ProposedSection { start: *start, ..Default::default() }).collect(), ..Default::default() };
            assert!(check_proposal(&proposal, &atoms, "me", &[]).0.is_none(), "{bad:?}");
        }
        let relations = vec![Relation { kind: "illustrates".into(), target: "other".into(), note: "화면".into() }, Relation { kind: "owns".into(), target: "other".into(), note: String::new() }, Relation { kind: "relates_to".into(), target: "ghost".into(), note: String::new() }];
        assert_eq!(clean_relations(&relations, "me", &["other".into()]).len(), 1);
        assert!(parse_proposal("정리안입니다\n```json\n{\"title\":\"t\",\"sections\":[]}\n```").is_ok());
    }

    #[test]
    fn names_suggest_a_role_and_images_are_never_curated() {
        assert_eq!(suggest_role("요구사항 명세.md", ""), "requirement");
        assert_eq!(suggest_role("notes.md", "# Product Requirements v2"), "requirement");
        assert_eq!(suggest_role("붙여넣은 글", "회의에서 나온 이야기"), "reference");
        assert!(!needs_curation_text("reference", &"## a\n\nb\n\n".repeat(5000)), "headed reference text needs no curator");
        assert!(needs_curation_text("requirement", &"## a\n\nb\n\n".repeat(5000)));
    }

    #[test]
    fn names_are_ascii_and_case_safe() {
        assert_eq!(slug("Requirements v2.md"), "requirements-v2");
        assert_eq!(slug("회의록.txt"), "");
        assert_eq!(slug("UI_Screen (1).PNG"), "ui-screen-1");
        let goal: Goal = serde_json::from_value(serde_json::json!({ "id": "G7f3A2-xx", "kind": "execution", "projectId": "p", "intent": "x", "status": "new", "report": "", "createdAt": "2026-10-14T01:00:00Z", "resumedFrom": null })).unwrap();
        assert_eq!(goal_folder(&goal), "work-references/2026-10-14_g7f3a2");
        assert!(is_reference_path("work-references/a/b.md") && !is_reference_path("src/work-references.md"));
    }

    #[test]
    fn base64_decodes_what_the_screen_sends() {
        assert_eq!(decode_base64("UklGRg==").unwrap(), b"RIFF");
        assert_eq!(decode_base64("YWJj").unwrap(), b"abc");
        assert!(decode_base64("@@").is_err());
    }

    #[test]
    fn webp_headers_give_the_real_size() {
        let mut lossless = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        // 1568 x 980: bits 0-13 width-1, 14-27 height-1.
        let packed: u32 = (1567) | (979 << 14);
        lossless.extend_from_slice(&packed.to_le_bytes());
        lossless.extend_from_slice(&[0; 8]);
        assert_eq!(webp_size(&lossless), Some((1568, 980)));
        let mut extended = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
        extended.extend_from_slice(&[0x1f, 0x06, 0x00, 0xd3, 0x03, 0x00]); // 1568-1, 980-1
        assert_eq!(webp_size(&extended), Some((1568, 980)));
        assert_eq!(webp_size(b"\x89PNG\r\n\x1a\n0000000000000000000000000000"), None);
    }

    fn store_and_project(kind: &str) -> (Store, Project, PathBuf) {
        let base = std::env::temp_dir().join(format!("utopia-refs-{}", crate::model::id()));
        std::fs::create_dir_all(base.join("project/.git/info")).unwrap();
        let store = Store::open(&base.join("db.sqlite")).unwrap();
        let project = Project { id: "p".into(), path: base.join("project").to_string_lossy().into_owned(), name: "p".into(), kind: kind.into(), review_loop_limit: 2 };
        (store, project, base)
    }

    fn goal(id: &str) -> Goal {
        serde_json::from_value(serde_json::json!({ "id": id, "kind": "execution", "projectId": "p", "intent": "README 링크 확인", "status": "new", "report": "", "createdAt": "2026-10-14T01:00:00Z", "resumedFrom": null })).unwrap()
    }

    #[test]
    fn attaching_writes_files_indexes_and_a_local_git_exclude_from_rules_only() {
        let (store, project, base) = store_and_project("git");
        let staging = base.join("staging");
        let short = stage_text(&store, &staging, &project, "notes.md", "# 링크\n\n- a\n", "file").unwrap();
        let long_text: String = (0..12).map(|i| format!("## 절{i}\n\n{}{}{}", para("본문", 60), para("본문", 60), para("본문", 60))).collect();
        let long = stage_text(&store, &staging, &project, "Requirements.md", &long_text, "file").unwrap();
        assert_eq!(long.curation, "none", "a long text with headings needs no curator");
        let mut lossless = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        lossless.extend_from_slice(&((99u32) | (49 << 14)).to_le_bytes());
        lossless.extend_from_slice(&[0; 8]);
        let image = stage_image(&store, &staging, &project, &lossless, ImageMeta { original_name: "로그인 화면.png".into(), original_bytes: 900_000, original_sha256: "a".repeat(64), caption: "오류 예시".into(), detail: false }).unwrap();
        let g = goal("g7f3a2xx");
        store.put("goal", &g.id, &g, None).unwrap();
        let attached = attach_to_goal(&store, &staging, &project, &g, &[short.id.clone(), long.id.clone(), image.id.clone()]).unwrap();
        let root = Path::new(&project.path);
        assert_eq!(attached.iter().map(|item| item.label.as_str()).collect::<Vec<_>>(), vec!["R1", "R2", "R3"]);
        assert_eq!(attached[0].path, "work-references/2026-10-14_g7f3a2/text/R1-notes.md");
        assert!(attached[1].sections.len() >= 3);
        assert_eq!(attached[1].sections.iter().map(|section| std::fs::read_to_string(root.join(&section.file)).unwrap().split_once("\n\n").unwrap().1.to_string()).collect::<String>(), long_text, "section files rebuild the original after their source line");
        assert!(root.join("work-references/2026-10-14_g7f3a2/text/R2-requirements/00-outline.md").is_file());
        assert_eq!(attached[2].path, "work-references/2026-10-14_g7f3a2/images/R3.webp", "a non-ASCII name falls back to the label");
        let index = std::fs::read_to_string(root.join("work-references/2026-10-14_g7f3a2/INDEX.md")).unwrap();
        assert!(index.contains("| R3 | 이미지 |") && index.contains("오류 예시") && index.contains("R2 §1"));
        assert!(std::fs::read_to_string(root.join("work-references/INDEX.md")).unwrap().contains("2026-10-14_g7f3a2/"));
        assert!(std::fs::read_to_string(root.join(".git/info/exclude")).unwrap().contains("/work-references/"));
        assert!(!staging.join(&short.id).exists(), "staging is cleaned");
        // Indexes are views: an edit is overwritten, a deleted file is marked missing.
        std::fs::write(root.join("work-references/2026-10-14_g7f3a2/INDEX.md"), "AI가 고친 인덱스").unwrap();
        std::fs::remove_file(root.join(&attached[0].path)).unwrap();
        write_indexes(&store, &project).unwrap();
        let index = std::fs::read_to_string(root.join("work-references/2026-10-14_g7f3a2/INDEX.md")).unwrap();
        assert!(index.contains("| R1 |"));
        assert_eq!(store.get::<Reference>("reference", &short.id).unwrap().unwrap().status, "missing");
        exclude_from_git(root, &project).unwrap();
        assert_eq!(std::fs::read_to_string(root.join(".git/info/exclude")).unwrap().matches("/work-references/").count(), 1, "added once");
    }

    #[test]
    fn cto_summaries_fill_only_rule_lines_and_purge_keeps_material_a_later_version_uses() {
        let (store, project, base) = store_and_project("folder");
        let staging = base.join("staging");
        let text = stage_text(&store, &staging, &project, "a.md", "첫 줄\n\n본문\n", "file").unwrap();
        let old = goal("oldgoal1");
        store.put("goal", &old.id, &old, None).unwrap();
        attach_to_goal(&store, &staging, &project, &old, &[text.id.clone()]).unwrap();
        assert_eq!(apply_cto_summaries(&store, &old.id, "분석...\n- 자료 요약 R1: 링크 34개와 확인 규칙\n자료 요약 R9: 없음").unwrap(), 1);
        let saved: Reference = store.get("reference", &text.id).unwrap().unwrap();
        assert_eq!((saved.summary.as_str(), saved.summary_source.as_str()), ("링크 34개와 확인 규칙", "cto"));
        // A newer version inherits; purging the old goal moves the record and keeps the file.
        let new = goal("newgoal2");
        store.put("goal", &new.id, &new, None).unwrap();
        let guidance: crate::model::GoalGuidance = serde_json::from_value(serde_json::json!({ "id": "gd", "goalId": old.id, "text": "바꿔", "status": "revised", "createdAt": "t", "replacementGoalId": new.id })).unwrap();
        store.put("goal_guidance", "gd", &guidance, None).unwrap();
        assert_eq!(for_goal(&store, &new.id).unwrap().len(), 1, "inherited");
        let heir = heir_of(&store, &old.id).unwrap();
        purge_goal(&store, &project, &old, heir).unwrap();
        let moved: Reference = store.get("reference", &text.id).unwrap().unwrap();
        assert_eq!(moved.goal_id.as_deref(), Some(new.id.as_str()));
        assert!(Path::new(&project.path).join(&moved.path).is_file());
        // Without an heir the folder goes.
        purge_goal(&store, &project, &new, None).unwrap();
        assert!(store.get::<Reference>("reference", &text.id).unwrap().is_none());
        assert!(!Path::new(&project.path).join(&moved.path).exists(), "the inherited file goes with its last owner");
    }
}
