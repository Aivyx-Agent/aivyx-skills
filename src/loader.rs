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
