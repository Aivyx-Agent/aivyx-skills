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

/// Which of the three possible locations a resolved `Skill`/
/// `SkillSummary` was loaded from: compiled into the binary (`Bundled`),
/// a per-user overlay directory (`User`), or a per-project overlay
/// directory (`Project`). See `SkillLoader::resolve_all` for the
/// priority order these are merged in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    Bundled,
    User,
    Project,
}

/// A fully-resolved skill, including its markdown `body` -- what
/// `SkillLoader::get` returns for a single, known-needed skill.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,
    pub source: SkillSource,
}

/// A lightweight, `body`-free view of a skill -- just enough to describe
/// it (e.g. for injection into a system prompt as a discovery list) and
/// to know which name to pass to `SkillLoader::get` for the full skill.
/// See `SkillLoader::list`.
#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: SkillSource,
}

/// Resolves the default library of `SKILL.md`-format skills: the 5
/// skills bundled into this crate at compile time, merged with an
/// optional caller-provided project and/or user overlay directory
/// (project overrides user overrides bundled, per same-named skill).
#[derive(Default)]
pub struct SkillLoader {
    project_dir: Option<std::path::PathBuf>,
    user_dir: Option<std::path::PathBuf>,
}

impl SkillLoader {
    /// Creates a loader with no overlay directories -- bundled skills
    /// only, until `with_project_dir`/`with_user_dir` are attached.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attaches a project overlay directory. `dir` must directly contain
    /// one subdirectory per skill, each holding a `SKILL.md` file (e.g.
    /// `dir/my-skill/SKILL.md`) -- the same shape as this crate's own
    /// bundled `skills/` directory. Do not pass a project root or any
    /// other directory that doesn't directly contain skill
    /// subdirectories: it will silently yield zero overlays, not an
    /// error. A `dir` that doesn't exist is likewise treated as "no
    /// overrides from this source," not an error.
    pub fn with_project_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.project_dir = Some(dir);
        self
    }

    /// Attaches a user overlay directory. Same expected shape and
    /// same-directory-vs-root pitfall as `with_project_dir`: `dir` must
    /// directly contain one subdirectory per skill, each holding a
    /// `SKILL.md` file (e.g. `dir/my-skill/SKILL.md`). A nonexistent
    /// `dir` is treated as "no overrides from this source," not an
    /// error.
    pub fn with_user_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.user_dir = Some(dir);
        self
    }

    /// Lists every resolved skill as a cheap, `body`-free `SkillSummary`
    /// (name + description only). The returned `Vec` is always sorted by
    /// `name`, deterministically, so repeated calls across runs produce
    /// an identical ordering -- this matters because callers typically
    /// inject this list into an LLM system prompt, where a reshuffling
    /// order would defeat prompt-prefix caching.
    pub fn list(&self) -> Vec<SkillSummary> {
        let mut summaries: Vec<SkillSummary> = self
            .resolve_all()
            .into_values()
            .map(|s| SkillSummary {
                name: s.name,
                description: s.description,
                source: s.source,
            })
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    /// Returns the fully-resolved skill (including `body`) for one name,
    /// applying project > user > bundled priority, or `None` if no
    /// source has a skill with that name.
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
        assert_eq!(
            names.len(),
            5,
            "expected exactly 5 bundled skills, got {names:?}"
        );
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

    /// Hardening test for the `debug-embed` fix: proves the bundled set
    /// is genuinely non-empty, not just correctly-shaped, so a
    /// regression (e.g. someone reverting the `debug-embed` feature, or
    /// any other build misconfiguration that makes `rust-embed` fall
    /// back to reading a stale absolute filesystem path) fails loudly
    /// here instead of silently shipping a library with zero skills.
    #[test]
    fn bundled_skills_is_never_empty() {
        assert!(
            !bundled_skills().is_empty(),
            "bundled_skills() returned zero skills -- rust-embed likely isn't embedding \
             content in this build (check the `debug-embed` feature in Cargo.toml)"
        );
    }

    /// Regression guard for deterministic ordering: `list()` must return
    /// its summaries sorted by name, every time, so repeated calls
    /// (including across separate process runs) produce an identical
    /// ordering for system-prompt injection / KV-cache prefix reuse.
    #[test]
    fn list_is_sorted_by_name() {
        let loader = SkillLoader::new();
        let names: Vec<String> = loader.list().into_iter().map(|s| s.name).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(
            names, sorted_names,
            "list() must return summaries sorted by name"
        );
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

    #[test]
    fn a_missing_project_dir_is_treated_as_no_overrides() {
        let loader = SkillLoader::new().with_project_dir(std::path::PathBuf::from(
            "/nonexistent/path/that/does/not/exist",
        ));
        let summaries = loader.list();
        assert_eq!(
            summaries.len(),
            5,
            "a missing project dir must not error or change the bundled count"
        );
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
        assert_eq!(
            summaries.len(),
            5,
            "an override replaces, it doesn't add a duplicate entry"
        );
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
                format!(
                    "---\nname: clear-communication\ndescription: {description}.\n---\n\nBody.\n"
                ),
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
        assert_eq!(
            summaries.len(),
            5,
            "a malformed overlay skill must be silently skipped"
        );
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
}
