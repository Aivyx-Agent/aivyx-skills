# Aivyx-Skills Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A working `aivyx-skills` crate: parse `SKILL.md`'s frontmatter format, embed the bundled default skills into the binary, and resolve a caller's optional project/user overlay directories on top of them with project > user > bundled priority.

**Architecture:** A small, dependency-light frontmatter parser (`src/frontmatter.rs`, no crate dependencies beyond `std`) feeds a `SkillLoader` (`src/loader.rs`) that merges three sources into one `HashMap<String, Skill>` per call: bundled content (embedded via the `rust-embed` crate from this repo's own `skills/` directory, already authored and committed) unioned with real filesystem reads of an optional user directory, then an optional project directory, each later source overwriting any name collision from an earlier one.

**Tech Stack:** Rust (edition 2024), `rust-embed = "8.12.0"` (the only new dependency).

## Global Constraints

- Bundled-skill content lives at `skills/` (repo root) — already authored and committed (5 real `SKILL.md` files: `systematic-debugging`, `brainstorming-and-scoping`, `writing-plans`, `self-review-before-done`, `clear-communication`). No task in this plan edits that content.
- A skill's frontmatter `name` field MUST match its containing directory name. For a **bundled** skill, a mismatch (or any parse failure) is a bug in this crate's own committed content — fail loudly (`panic!`), never silently. For a **project/user overlay** skill, a mismatch or parse failure is external, possibly-malformed input — skip it silently, never panic and never fail the whole `list()`/`get()` call.
- A missing project/user overlay directory (doesn't exist, or any other read error) is treated as "no skills from this source," not an error.
- Resolution priority on a name collision: project > user > bundled (project always wins).
- `SkillLoader::new()` and its builder methods (`with_project_dir`/`with_user_dir`) are infallible — no `Result` in that part of the public API.
- File-scoped `rustfmt --edition 2024 <path>` only.

---

### Task 1: Frontmatter parser

**Files:**
- Create: `src/frontmatter.rs`
- Modify: `src/lib.rs` (add `mod frontmatter;`)

**Interfaces:**
- Produces: `pub(crate) fn parse_skill_md(raw: &str) -> Result<(String, String, String), FrontmatterError>` — returns `(name, description, body)`. Consumed by Task 2.
- Produces: `pub(crate) enum FrontmatterError { MissingOpeningDelimiter, MissingClosingDelimiter, MissingField(&'static str) }`, implementing `std::fmt::Display` and `std::error::Error`.

- [ ] **Step 1: Write the failing tests**

Create `src/frontmatter.rs` with just the test module first:

```rust
//! Parsing for the SKILL.md frontmatter format: a `---`-delimited header
//! with exactly two required scalar fields (`name`, `description`),
//! followed by a markdown body. Deliberately NOT a full YAML parser --
//! the schema is two flat string fields, so a minimal, dependency-free
//! line-based parser is used instead of pulling in a general YAML crate,
//! matching this workspace's established preference for keeping small
//! shared crates dependency-light (see `aivyx-injection-guard`'s own
//! "no dependencies -- pure std").

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_skill_md() {
        let raw = "---\nname: systematic-debugging\ndescription: A structured methodology.\n---\n\n# Systematic Debugging\n\nBody text here.\n";
        let (name, description, body) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "systematic-debugging");
        assert_eq!(description, "A structured methodology.");
        assert_eq!(body, "# Systematic Debugging\n\nBody text here.\n");
    }

    #[test]
    fn only_splits_on_the_first_colon_in_a_value() {
        let raw = "---\nname: clear-communication\ndescription: Use when: writing anything user-facing.\n---\n\nBody.\n";
        let (_, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(description, "Use when: writing anything user-facing.");
    }

    #[test]
    fn tolerates_unknown_frontmatter_fields() {
        let raw = "---\nname: a-skill\ndescription: A description.\nauthor: someone\n---\n\nBody.\n";
        let (name, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "a-skill");
        assert_eq!(description, "A description.");
    }

    #[test]
    fn rejects_a_file_with_no_opening_delimiter() {
        let raw = "name: a-skill\ndescription: A description.\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingOpeningDelimiter
        );
    }

    #[test]
    fn rejects_a_file_with_no_closing_delimiter() {
        let raw = "---\nname: a-skill\ndescription: A description.\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingClosingDelimiter
        );
    }

    #[test]
    fn rejects_frontmatter_missing_the_name_field() {
        let raw = "---\ndescription: A description.\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingField("name")
        );
    }

    #[test]
    fn rejects_frontmatter_missing_the_description_field() {
        let raw = "---\nname: a-skill\n---\n\nBody.\n";
        assert_eq!(
            parse_skill_md(raw).unwrap_err(),
            FrontmatterError::MissingField("description")
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail to compile**

Run: `cargo test parse_skill_md`
Expected: FAIL — `parse_skill_md`/`FrontmatterError` don't exist yet.

- [ ] **Step 3: Implement the parser**

In `src/frontmatter.rs`, above the `#[cfg(test)] mod tests` block, add:

```rust
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrontmatterError {
    MissingOpeningDelimiter,
    MissingClosingDelimiter,
    MissingField(&'static str),
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontmatterError::MissingOpeningDelimiter => {
                write!(f, "file does not start with a `---` frontmatter delimiter")
            }
            FrontmatterError::MissingClosingDelimiter => {
                write!(f, "frontmatter has no closing `---` delimiter")
            }
            FrontmatterError::MissingField(field) => {
                write!(f, "frontmatter is missing required field `{field}`")
            }
        }
    }
}

impl std::error::Error for FrontmatterError {}

/// Parses a full `SKILL.md` file's contents into `(name, description,
/// body)`. `body` is the markdown content after the closing `---`, with
/// leading blank lines trimmed (trailing content is preserved as-is).
pub(crate) fn parse_skill_md(raw: &str) -> Result<(String, String, String), FrontmatterError> {
    let after_open = raw
        .strip_prefix("---\n")
        .ok_or(FrontmatterError::MissingOpeningDelimiter)?;

    let close_pos = after_open
        .find("\n---\n")
        .ok_or(FrontmatterError::MissingClosingDelimiter)?;

    let frontmatter_block = &after_open[..close_pos];
    let rest = &after_open[close_pos + "\n---\n".len()..];

    let mut name = None;
    let mut description = None;
    for line in frontmatter_block.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "name" => name = Some(value.trim().to_string()),
            "description" => description = Some(value.trim().to_string()),
            _ => {}
        }
    }

    let name = name.ok_or(FrontmatterError::MissingField("name"))?;
    let description = description.ok_or(FrontmatterError::MissingField("description"))?;
    let body = rest.trim_start_matches('\n').to_string();

    Ok((name, description, body))
}
```

- [ ] **Step 4: Wire the module into `src/lib.rs`**

Create `src/lib.rs` with:

```rust
mod frontmatter;
```

(Task 2 adds the rest of `lib.rs`'s content and turns this into a real public crate root.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test parse_skill_md`
Expected: all 7 tests pass.

- [ ] **Step 6: Format and commit**

```bash
rustfmt --edition 2024 src/frontmatter.rs
rustfmt --edition 2024 src/lib.rs
git add src/frontmatter.rs src/lib.rs
git commit -m "feat: SKILL.md frontmatter parser"
```

---

### Task 2: `SkillLoader` core + bundled-skill embedding

**Files:**
- Create: `src/loader.rs`
- Modify: `src/lib.rs` (add `mod loader;` and public re-exports)
- Modify: `Cargo.toml` (add `rust-embed` dependency)

**Interfaces:**
- Consumes: `crate::frontmatter::parse_skill_md` (Task 1).
- Produces: `pub struct Skill { pub name: String, pub description: String, pub body: String, pub source: SkillSource }`, `pub enum SkillSource { Bundled, User, Project }`, `pub struct SkillSummary { pub name: String, pub description: String, pub source: SkillSource }`, `pub struct SkillLoader` with `pub fn new() -> Self` and (stubbed in this task, completed in Task 3) `pub fn list(&self) -> Vec<SkillSummary>` / `pub fn get(&self, name: &str) -> Option<Skill>` that for THIS task only need to see bundled skills (Task 3 adds the overlay directories and the `with_project_dir`/`with_user_dir` builders) — consumed by Task 3.

- [ ] **Step 1: Add the `rust-embed` dependency**

In `Cargo.toml`, find this exact line:

```toml
[dependencies]
```

Replace it with:

```toml
[dependencies]
rust-embed = "8.12.0"
```

- [ ] **Step 2: Write the failing tests**

Create `src/loader.rs` with:

```rust
//! `SkillLoader`: merges the bundled default skills (embedded into the
//! binary from this repo's own `skills/` directory) with an optional
//! caller-provided project and/or user overlay directory. See
//! `docs/superpowers/specs/2026-09-23-aivyx-skills-design.md` for the
//! full design.

use rust_embed::RustEmbed;

use crate::frontmatter::parse_skill_md;

#[derive(RustEmbed)]
#[folder = "skills/"]
struct BundledSkills;

/// Who a resolved `Skill`/`SkillSummary` came from -- `Project` and
/// `User` only ever appear once `SkillLoader::with_project_dir`/
/// `with_user_dir` are used (Task 3); this task only ever produces
/// `Bundled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Bundled,
    User,
    Project,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: SkillSource,
}

#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
}

#[derive(Default)]
pub struct SkillLoader {}

impl SkillLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<SkillSummary> {
        bundled_skills()
            .into_iter()
            .map(|s| SkillSummary {
                name: s.name,
                description: s.description,
                source: s.source,
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Skill> {
        bundled_skills().into_iter().find(|s| s.name == name)
    }
}

/// Parses every bundled `SKILL.md`, panicking on a parse failure or a
/// name/directory mismatch -- both are only reachable via a bug in this
/// crate's own committed content, never from runtime/user data, and
/// `all_bundled_skills_parse_and_match_their_directory_name` below is
/// specifically designed to catch either before any release.
fn bundled_skills() -> Vec<Skill> {
    let mut skills = Vec::new();
    for path in BundledSkills::iter() {
        let Some(dir_name) = path.strip_suffix("/SKILL.md") else {
            continue;
        };
        let file = BundledSkills::get(&path).unwrap_or_else(|| {
            panic!("bundled skill {path} listed by iter() but get() returned None")
        });
        let raw = std::str::from_utf8(&file.data)
            .unwrap_or_else(|e| panic!("bundled skill {path} is not valid UTF-8: {e}"));
        let (name, description, body) = parse_skill_md(raw)
            .unwrap_or_else(|e| panic!("bundled skill {path} failed to parse: {e}"));
        assert_eq!(
            name, dir_name,
            "bundled skill {path}'s frontmatter name {name:?} must match its directory name {dir_name:?}"
        );
        skills.push(Skill {
            name,
            description,
            body,
            source: SkillSource::Bundled,
        });
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single most important test in this crate: proves every
    /// bundled skill committed to `skills/` genuinely parses under
    /// Task 1's frontmatter parser and that its `name` field matches its
    /// own directory -- if this ever fails, `bundled_skills()` would
    /// otherwise panic at runtime for every single consumer of this
    /// crate, so catching it here, in CI, is the whole point.
    #[test]
    fn all_bundled_skills_parse_and_match_their_directory_name() {
        let skills = bundled_skills();
        let names: Vec<&str> = skills.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names.len(), 5, "expected exactly 5 bundled skills, got {names:?}");
        for expected in [
            "systematic-debugging",
            "brainstorming-and-scoping",
            "writing-plans",
            "self-review-before-done",
            "clear-communication",
        ] {
            assert!(
                names.contains(&expected),
                "expected bundled skill {expected:?} to be present, got {names:?}"
            );
        }
        for skill in &skills {
            assert_eq!(skill.source, SkillSource::Bundled);
            assert!(!skill.description.is_empty());
            assert!(!skill.body.is_empty());
        }
    }

    #[test]
    fn list_returns_a_summary_for_every_bundled_skill() {
        let loader = SkillLoader::new();
        let summaries = loader.list();
        assert_eq!(summaries.len(), 5);
        assert!(
            summaries
                .iter()
                .any(|s| s.name == "systematic-debugging" && s.source == SkillSource::Bundled)
        );
    }

    #[test]
    fn get_returns_the_full_skill_body_for_a_known_name() {
        let loader = SkillLoader::new();
        let skill = loader.get("systematic-debugging").unwrap();
        assert_eq!(skill.name, "systematic-debugging");
        assert_eq!(skill.source, SkillSource::Bundled);
        assert!(skill.body.contains("Reproduce"));
    }

    #[test]
    fn get_returns_none_for_an_unknown_name() {
        let loader = SkillLoader::new();
        assert!(loader.get("does-not-exist").is_none());
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail to compile**

Run: `cargo test --lib loader`
Expected: FAIL — `src/loader.rs` isn't wired into `src/lib.rs` yet, so nothing in it is reachable/compiled as part of the crate.

- [ ] **Step 4: Wire the module into `src/lib.rs` and add public re-exports**

Replace `src/lib.rs`'s entire contents with:

```rust
mod frontmatter;
mod loader;

pub use loader::{Skill, SkillLoader, SkillSource, SkillSummary};
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test`
Expected: all tests pass (Task 1's 7 frontmatter tests + this task's 4 loader tests = 11 total).

- [ ] **Step 6: Format and commit**

```bash
rustfmt --edition 2024 src/loader.rs
rustfmt --edition 2024 src/lib.rs
rustfmt --edition 2024 Cargo.toml
git add Cargo.toml src/loader.rs src/lib.rs Cargo.lock
git commit -m "feat: SkillLoader with bundled-skill embedding via rust-embed"
```

(`Cargo.lock` will be created/updated by the `cargo test` run in Step 5 — include it, this is a binary-adjacent library crate other repos will pin by git rev, so a committed lockfile keeps builds reproducible, matching this workspace's other small shared crates.)

---

### Task 3: Project/user overlay + priority resolution

**Files:**
- Modify: `src/loader.rs`

**Interfaces:**
- Consumes: `Skill`, `SkillSource`, `bundled_skills()` (Task 2, same file).
- Produces: `SkillLoader::with_project_dir(self, dir: PathBuf) -> Self`, `SkillLoader::with_user_dir(self, dir: PathBuf) -> Self` — the crate's full, final public API, matching the design spec exactly.

- [ ] **Step 1: Write the failing tests**

In `src/loader.rs`'s `#[cfg(test)] mod tests` block, add these tests (after the existing ones):

```rust
#[test]
fn a_missing_project_dir_is_treated_as_no_overrides() {
    let loader = SkillLoader::new().with_project_dir(std::path::PathBuf::from("/nonexistent/path/that/does/not/exist"));
    let summaries = loader.list();
    assert_eq!(summaries.len(), 5, "a missing project dir must not error or change the bundled count");
}

#[test]
fn a_project_dir_skill_overrides_a_bundled_skill_of_the_same_name() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("systematic-debugging");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: systematic-debugging\ndescription: A project-local override.\n---\n\nOverridden body.\n",
    )
    .unwrap();

    let loader = SkillLoader::new().with_project_dir(dir.path().to_path_buf());
    let skill = loader.get("systematic-debugging").unwrap();
    assert_eq!(skill.source, SkillSource::Project);
    assert_eq!(skill.description, "A project-local override.");
    assert_eq!(skill.body, "Overridden body.\n");

    let summaries = loader.list();
    assert_eq!(summaries.len(), 5, "an override replaces, it doesn't add a duplicate entry");
}

#[test]
fn a_project_dir_override_wins_over_a_user_dir_override_of_the_same_name() {
    let project_dir = tempfile::tempdir().unwrap();
    let user_dir = tempfile::tempdir().unwrap();

    for (dir, description) in [(&project_dir, "from project"), (&user_dir, "from user")] {
        let skill_dir = dir.path().join("clear-communication");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: clear-communication\ndescription: {description}.\n---\n\nBody.\n"),
        )
        .unwrap();
    }

    let loader = SkillLoader::new()
        .with_user_dir(user_dir.path().to_path_buf())
        .with_project_dir(project_dir.path().to_path_buf());
    let skill = loader.get("clear-communication").unwrap();
    assert_eq!(skill.source, SkillSource::Project);
    assert_eq!(skill.description, "from project.");
}

#[test]
fn a_new_skill_in_an_overlay_dir_is_added_not_just_used_for_overrides() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("a-brand-new-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: a-brand-new-skill\ndescription: Not one of the bundled five.\n---\n\nBody.\n",
    )
    .unwrap();

    let loader = SkillLoader::new().with_project_dir(dir.path().to_path_buf());
    let summaries = loader.list();
    assert_eq!(summaries.len(), 6);
    assert!(loader.get("a-brand-new-skill").is_some());
}

#[test]
fn a_malformed_overlay_skill_is_skipped_not_treated_as_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("broken-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), "not valid frontmatter at all").unwrap();

    let loader = SkillLoader::new().with_project_dir(dir.path().to_path_buf());
    let summaries = loader.list();
    assert_eq!(summaries.len(), 5, "a malformed overlay skill must be silently skipped");
    assert!(loader.get("broken-skill").is_none());
}

#[test]
fn an_overlay_skill_whose_name_does_not_match_its_directory_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("the-directory-name");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: a-different-name\ndescription: Mismatched on purpose.\n---\n\nBody.\n",
    )
    .unwrap();

    let loader = SkillLoader::new().with_project_dir(dir.path().to_path_buf());
    let summaries = loader.list();
    assert_eq!(summaries.len(), 5);
    assert!(loader.get("the-directory-name").is_none());
    assert!(loader.get("a-different-name").is_none());
}
```

- [ ] **Step 2: Add `tempfile` as a dev-dependency**

In `Cargo.toml`, find this exact line:

```toml
[dependencies]
rust-embed = "8.12.0"
```

Replace it with:

```toml
[dependencies]
rust-embed = "8.12.0"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib`
Expected: FAIL to compile — `with_project_dir`/`with_user_dir` don't exist yet.

- [ ] **Step 4: Implement the overlay + resolution logic**

Task 2's Step 6 ran `rustfmt` on this file, so whitespace/line-wrapping
may have shifted slightly from how it's quoted below (rustfmt should be a
no-op on this particular code, but don't assume it byte-for-byte matches
without checking). Read the real current `src/loader.rs` first to confirm
the exact text before editing; if formatting differs from what's quoted,
match on the real content and apply the same logical change — the new
struct fields, the two builder methods, and the `resolve_all` method
shown below are the actual requirement, not the surrounding whitespace.

In `src/loader.rs`, find this block (or its current, possibly
reformatted, equivalent):

```rust
#[derive(Default)]
pub struct SkillLoader {}

impl SkillLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<SkillSummary> {
        bundled_skills()
            .into_iter()
            .map(|s| SkillSummary {
                name: s.name,
                description: s.description,
                source: s.source,
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Skill> {
        bundled_skills().into_iter().find(|s| s.name == name)
    }
}
```

Replace it with:

```rust
#[derive(Default)]
pub struct SkillLoader {
    project_dir: Option<std::path::PathBuf>,
    user_dir: Option<std::path::PathBuf>,
}

impl SkillLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_project_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.project_dir = Some(dir);
        self
    }

    pub fn with_user_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.user_dir = Some(dir);
        self
    }

    pub fn list(&self) -> Vec<SkillSummary> {
        self.resolve_all()
            .into_values()
            .map(|s| SkillSummary {
                name: s.name,
                description: s.description,
                source: s.source,
            })
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<Skill> {
        self.resolve_all().remove(name)
    }

    /// Builds the full, priority-resolved skill map: bundled first, then
    /// user overrides, then project overrides -- each later source
    /// replacing any earlier entry with the same name in the
    /// `HashMap`, so project always wins over user, which always wins
    /// over bundled.
    fn resolve_all(&self) -> std::collections::HashMap<String, Skill> {
        let mut skills = std::collections::HashMap::new();
        for skill in bundled_skills() {
            skills.insert(skill.name.clone(), skill);
        }
        if let Some(dir) = &self.user_dir {
            for skill in overlay_skills(dir, SkillSource::User) {
                skills.insert(skill.name.clone(), skill);
            }
        }
        if let Some(dir) = &self.project_dir {
            for skill in overlay_skills(dir, SkillSource::Project) {
                skills.insert(skill.name.clone(), skill);
            }
        }
        skills
    }
}
```

Then, immediately after the existing `bundled_skills()` function (before the `#[cfg(test)]` block), add:

```rust
/// Scans `dir` for `<name>/SKILL.md` entries, skipping (not erroring on)
/// anything that fails to parse or whose frontmatter `name` doesn't
/// match its own directory name -- unlike bundled skills, these come
/// from outside this crate's control, so a malformed one degrades
/// gracefully rather than panicking. A `dir` that doesn't exist, or any
/// other read error, is silently treated as "no skills from this
/// source."
fn overlay_skills(dir: &std::path::Path, source: SkillSource) -> Vec<Skill> {
    let mut skills = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return skills;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(raw) = std::fs::read_to_string(path.join("SKILL.md")) else {
            continue;
        };
        let Ok((name, description, body)) = parse_skill_md(&raw) else {
            continue;
        };
        if name != dir_name {
            continue;
        }
        skills.push(Skill {
            name,
            description,
            body,
            source: source.clone(),
        });
    }
    skills
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test`
Expected: all tests pass (Task 1's 7 + Task 2's 4 + this task's 6 = 17 total).

- [ ] **Step 6: Format, lint, and commit**

```bash
rustfmt --edition 2024 src/loader.rs
rustfmt --edition 2024 Cargo.toml
cargo clippy --all-targets
git add Cargo.toml Cargo.lock src/loader.rs
git commit -m "feat: project/user overlay directories with priority resolution"
```

Expected clippy output: clean, zero warnings.

---

## Self-Review Notes

**Spec coverage:** Decision 1 (small Rust crate, not content-only) → the whole plan operates inside the existing `aivyx-skills` crate scaffold. Decision 2 (repo layout: `skills/` already committed; `src/lib.rs`/`loader.rs`/`frontmatter.rs`) → Tasks 1-2. Decision 3 (`SKILL.md` format: `name`/`description` frontmatter, `name` must match directory) → Task 1 (parser) + Task 2/3 (the match-check, panic for bundled / skip for overlay). Decision 4 (public API: `Skill`/`SkillSource`/`SkillSummary`/`SkillLoader` with `new`/`with_project_dir`/`with_user_dir`/`list`/`get`) → Task 2 (bundled-only) then Task 3 (full). Decision 5 (project > user > bundled priority, missing dir = no-op) → Task 3. Decision 6 (`rust-embed` for bundled embedding) → Task 2. Decision 7 (the five real skills) — already authored and committed before this plan; Task 2's `all_bundled_skills_parse_and_match_their_directory_name` test is the proof they're genuinely wired in correctly. "What this spec does not decide" items are all genuinely untouched: no consumption code for either product, no change to `aivyx-pa`'s own Skill system, no `aivyx-hub` change, no GitHub/CI setup, no additional skills beyond the five, no skill-composition mechanism.

**Global Constraints deviation:** none — bundled content untouched by any task, panic-vs-skip behavior implemented exactly as specified in both Task 2 (bundled) and Task 3 (overlay), missing-dir-is-a-no-op tested explicitly (Task 3's first new test), project > user > bundled priority tested explicitly (Task 3's third new test), `new()`/builders stay infallible (no `Result` anywhere in the public API), only file-scoped `rustfmt`.

**Placeholder scan:** no TBD/TODO; every step shows complete, real code, including every test's full body — this plan has no genuinely investigative steps (unlike several plans in the sibling `aivyx-coder` repo's own history), since this is a brand-new crate with no pre-existing code whose exact current text needs to be independently re-verified before writing a find/replace block.

**Type/interface consistency check:** `parse_skill_md(raw: &str) -> Result<(String, String, String), FrontmatterError>` (Task 1) is called identically in Task 2's `bundled_skills()` and Task 3's `overlay_skills()`. `Skill { name, description, body, source }`/`SkillSource::{Bundled, User, Project}`/`SkillSummary { name, description, source }` (Task 2) are constructed identically in Task 3's `overlay_skills()` and consumed identically in `resolve_all()`. `SkillLoader`'s field set grows from `{}` (Task 2) to `{project_dir, user_dir}` (Task 3) with `#[derive(Default)]` still valid for both shapes (both are `Option`-only/empty structs, both have a sensible all-`None`/empty default) — confirmed no conflict between the two tasks' versions of the struct definition, since Task 3's Step 4 replaces the entire `impl SkillLoader` block (including the struct definition) from Task 2's version, not diffing against a stale partial state.
