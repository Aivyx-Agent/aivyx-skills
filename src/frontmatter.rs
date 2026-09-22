//! Parsing for the SKILL.md frontmatter format: a `---`-delimited header
//! with exactly two required scalar fields (`name`, `description`),
//! followed by a markdown body. Deliberately NOT a full YAML parser --
//! the schema is two flat string fields, so a minimal, dependency-free
//! line-based parser is used instead of pulling in a general YAML crate,
//! matching this workspace's established preference for keeping small
//! shared crates dependency-light (see `aivyx-injection-guard`'s own
//! "no dependencies -- pure std").

use std::fmt;

// The shared `Missing` prefix is intentional, not an oversight: all three
// variants are the same underlying condition (something required wasn't
// present), and that shared vocabulary is clearer here than inventing
// three unrelated-sounding names just to satisfy the lint.
#[allow(clippy::enum_variant_names)]
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
    // Normalize CRLF to LF up front so every delimiter/line check below --
    // in the frontmatter block AND the body -- works identically whether
    // the file was authored/checked-out with `\n` or `\r\n` line endings.
    let raw = raw.replace("\r\n", "\n");

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
        let raw =
            "---\nname: a-skill\ndescription: A description.\nauthor: someone\n---\n\nBody.\n";
        let (name, description, _) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "a-skill");
        assert_eq!(description, "A description.");
    }

    #[test]
    fn tolerates_crlf_line_endings() {
        let raw = "---\r\nname: systematic-debugging\r\ndescription: A structured methodology.\r\n---\r\n\r\n# Systematic Debugging\r\n\r\nBody text here.\r\n";
        let (name, description, body) = parse_skill_md(raw).unwrap();
        assert_eq!(name, "systematic-debugging");
        assert_eq!(description, "A structured methodology.");
        assert_eq!(body, "# Systematic Debugging\n\nBody text here.\n");
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
