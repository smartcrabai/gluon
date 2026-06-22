use std::path::Path;

use anyhow::{Context, Result};

const BINDS_START: &str = "// <gluon:binds>";
const BINDS_END: &str = "// </gluon:binds>";
const BIND_OPEN_PREFIX: &str = "<gluon:bind:";
const BIND_CLOSE_PREFIX: &str = "</gluon:bind:";
/// Character class accepted for a bind key. Matches the CLI generators
/// (`usecase:foo`, `domain:foo`); explicitly rejects `.`, `/`, and `..` so a
/// stray bind key cannot pretend to be a path segment.
fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '-')
}

/// Insert (or replace) a named bind block inside the `// <gluon:binds>` ...
/// `// </gluon:binds>` region of `wiring.rs`.
///
/// The block is written as:
/// ```text
/// // <gluon:bind:{key}>
/// {line}
/// // </gluon:bind:{key}>
/// ```
///
/// Blocks are kept sorted by `key`. If a block with the same `key` already
/// exists, it is replaced in place.
///
/// Returns `Ok(())` and leaves the file untouched if the binds region is not
/// found.
pub fn insert_bind(wiring_path: &Path, key: &str, line: &str) -> Result<()> {
    let content = std::fs::read_to_string(wiring_path)
        .with_context(|| format!("failed to read wiring file: {}", wiring_path.display()))?;

    let Some((before, binds_body, after, indent)) = split_binds_region(&content) else {
        return Ok(());
    };

    let mut blocks = parse_blocks(binds_body)
        .map_err(|e| anyhow::anyhow!("malformed wiring file {}: {e}", wiring_path.display()))?;
    let new_block = Block {
        key: key.to_owned(),
        body: line.to_owned(),
    };
    if let Some(pos) = blocks.iter().position(|b| b.key == key) {
        blocks[pos] = new_block;
    } else {
        blocks.push(new_block);
    }
    blocks.sort_by(|a, b| a.key.cmp(&b.key));

    let rendered = render_blocks(&blocks, &indent);
    let new_content = format!("{before}{rendered}{after}");
    std::fs::write(wiring_path, new_content)
        .with_context(|| format!("failed to write wiring file: {}", wiring_path.display()))?;
    Ok(())
}

#[derive(Debug)]
struct Block {
    key: String,
    body: String,
}

/// Split the file into `(before_binds_region, binds_body, after_binds_region, indent)`.
///
/// `before` ends with the `// <gluon:binds>` line and trailing newline.
/// `after`  starts with the indent of `// </gluon:binds>`.
/// `binds_body` is everything between them (exclusive of those marker lines).
/// `indent` is the leading whitespace of `// <gluon:binds>`.
fn split_binds_region(content: &str) -> Option<(String, &str, String, String)> {
    let start_idx = content.find(BINDS_START)?;

    // Search for the end marker only *after* the start marker so that an
    // accidental `// </gluon:binds>` string inside the body does not
    // prematurely truncate the region and silently delete bindings.
    let after_start = start_idx + BINDS_START.len();
    let end_idx = content[after_start..]
        .find(BINDS_END)
        .map(|i| i + after_start)?;
    if end_idx <= start_idx {
        return None;
    }

    // Compute indent of the start marker by walking back from start_idx to the
    // previous newline (or start of file).
    let line_start = content[..start_idx].rfind('\n').map_or(0, |nl| nl + 1);
    let indent: String = content[line_start..start_idx].to_owned();

    // `before` extends up to and including the newline that terminates the
    // start marker line.
    let after_start_marker = start_idx + BINDS_START.len();
    let body_start = match content[after_start_marker..].find('\n') {
        Some(off) => after_start_marker + off + 1,
        None => after_start_marker,
    };
    let before = content[..body_start].to_owned();

    // `after` starts at the indent of the end marker (i.e. the start of the
    // line that contains it).
    let end_line_start = content[..end_idx].rfind('\n').map_or(0, |nl| nl + 1);
    let after = content[end_line_start..].to_owned();

    let binds_body = &content[body_start..end_line_start];
    Some((before, binds_body, after, indent))
}

/// Parse the body of the binds region into a list of named blocks.
///
/// Hand-rolled (rather than regex-based) because the `regex` crate does not
/// support backreferences, and we need the close marker to match the
/// previously-captured key.
///
/// Returns `Err` when an open marker has no matching close marker, indicating
/// a manually corrupted wiring file. The caller must NOT rewrite the file on
/// error to prevent silently deleting bindings.
fn parse_blocks(body: &str) -> Result<Vec<Block>, String> {
    let mut out = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(key) = open_marker_key(lines[i]) {
            let close = format!("{BIND_CLOSE_PREFIX}{key}>");
            let mut body_lines: Vec<&str> = Vec::new();
            let mut j = i + 1;
            let mut found = false;
            while j < lines.len() {
                if line_is_close_marker(lines[j], &close) {
                    found = true;
                    break;
                }
                body_lines.push(lines[j]);
                j += 1;
            }
            if found {
                out.push(Block {
                    key,
                    body: body_lines.join("\n"),
                });
                i = j + 1;
                continue;
            }
            return Err(format!(
                "unclosed bind block: missing `// {BIND_CLOSE_PREFIX}{key}>` -- \
                 fix wiring.rs manually before re-running gluon generate"
            ));
        }
        i += 1;
    }
    Ok(out)
}

/// If `line` is a `// <gluon:bind:KEY>` marker, return the key.
fn open_marker_key(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("//")?.trim_start();
    let rest = rest.strip_prefix(BIND_OPEN_PREFIX)?;
    let key_end = rest.find('>')?;
    let key = &rest[..key_end];
    let trailing = rest[key_end + 1..].trim();
    if !trailing.is_empty() || key.is_empty() || !key.chars().all(is_key_char) {
        return None;
    }
    Some(key.to_owned())
}

/// True iff `line` is the close marker `// </gluon:bind:KEY>` whose body equals
/// `expected_marker` (`"</gluon:bind:KEY>"`).
fn line_is_close_marker(line: &str, expected_marker: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("//") else {
        return false;
    };
    rest.trim() == expected_marker
}

/// Render a list of blocks back to source text. Each block becomes:
///
/// ```text
/// {indent}// <gluon:bind:{key}>
/// {body lines reindented to `indent`}
/// {indent}// </gluon:bind:{key}>
/// ```
///
/// The result always ends with a trailing newline so the caller can paste the
/// closing `// </gluon:binds>` line immediately after.
fn render_blocks(blocks: &[Block], indent: &str) -> String {
    if blocks.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for block in blocks {
        out.push_str(indent);
        out.push_str("// <gluon:bind:");
        out.push_str(&block.key);
        out.push_str(">\n");
        for line in block.body.lines() {
            if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(indent);
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(indent);
        out.push_str("// </gluon:bind:");
        out.push_str(&block.key);
        out.push_str(">\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_blocks_empty_body() {
        assert!(parse_blocks("").unwrap().is_empty());
    }

    #[test]
    fn parse_blocks_single_block() {
        let body = "    // <gluon:bind:usecase:list_users>\n    builder = builder.bind(...);\n    // </gluon:bind:usecase:list_users>\n";
        let blocks = parse_blocks(body).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].key, "usecase:list_users");
        assert!(blocks[0].body.contains("builder = builder.bind"));
    }

    #[test]
    fn parse_blocks_two_sorted_blocks() {
        let body = concat!(
            "    // <gluon:bind:domain:user>\n",
            "    builder = builder.bind(D);\n",
            "    // </gluon:bind:domain:user>\n",
            "    // <gluon:bind:usecase:list_users>\n",
            "    builder = builder.bind(U);\n",
            "    // </gluon:bind:usecase:list_users>\n",
        );
        let blocks = parse_blocks(body).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].key, "domain:user");
        assert_eq!(blocks[1].key, "usecase:list_users");
    }

    #[test]
    fn parse_blocks_rejects_disallowed_key_chars() {
        // Bad key chars are not recognized as an open marker, so the block is
        // skipped without an error (not an unclosed block).
        let body = "// <gluon:bind:bad/key>\nbody\n// </gluon:bind:bad/key>\n";
        assert!(parse_blocks(body).unwrap().is_empty());
    }

    #[test]
    fn parse_blocks_errors_on_unclosed_block() {
        // A valid open marker without a matching close marker must return Err
        // so the caller does not rewrite the file and silently delete the binding.
        let body = "// <gluon:bind:usecase:foo>\nbuilder = builder.bind(Foo);\n";
        let err = parse_blocks(body).unwrap_err();
        assert!(err.contains("usecase:foo"), "err: {err}");
    }

    // split_binds_region cases.

    #[test]
    fn split_binds_region_missing_markers() {
        assert!(split_binds_region("no markers here").is_none());
        assert!(split_binds_region("// <gluon:binds>\nonly open\n").is_none());
        assert!(split_binds_region("// </gluon:binds>\nonly close\n").is_none());
    }

    #[test]
    fn split_binds_region_end_before_start_returns_none() {
        // End marker appears before start marker.
        let content = "// </gluon:binds>\n// <gluon:binds>\n";
        assert!(split_binds_region(content).is_none());
    }

    #[test]
    fn split_binds_region_tab_indent() {
        let content = "fn f() {\n\t// <gluon:binds>\n\t// </gluon:binds>\n}\n";
        let (_before, _body, _after, indent) = split_binds_region(content).unwrap();
        assert_eq!(indent, "\t");
    }

    #[test]
    fn split_binds_region_no_indent_at_file_start() {
        let content = "// <gluon:binds>\n// </gluon:binds>\n";
        let (_before, _body, _after, indent) = split_binds_region(content).unwrap();
        assert_eq!(indent, "");
    }

    #[test]
    fn split_binds_region_empty_body_when_markers_adjacent() {
        let content = "// <gluon:binds>\n// </gluon:binds>\n";
        let (_before, body, _after, _indent) = split_binds_region(content).unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn split_binds_region_ignores_end_marker_before_start() {
        // An accidental `// </gluon:binds>` that appears before `// <gluon:binds>`
        // must not be used as the end marker; the function must search only after
        // the start marker so that body content containing that string is safe.
        let content = "// </gluon:binds>\n// <gluon:binds>\nbody\n// </gluon:binds>\n";
        let (_before, body, _after, _indent) = split_binds_region(content).unwrap();
        assert_eq!(body, "body\n");
    }

    // open_marker_key cases.

    #[test]
    fn open_marker_key_simple() {
        assert_eq!(
            open_marker_key("// <gluon:bind:foo>"),
            Some("foo".to_owned())
        );
    }

    #[test]
    fn open_marker_key_with_indent_and_colon_in_key() {
        assert_eq!(
            open_marker_key("    // <gluon:bind:usecase:list_users>"),
            Some("usecase:list_users".to_owned())
        );
    }

    #[test]
    fn open_marker_key_empty_key_returns_none() {
        assert_eq!(open_marker_key("// <gluon:bind:>"), None);
    }

    #[test]
    fn open_marker_key_rejects_disallowed_chars() {
        assert_eq!(open_marker_key("// <gluon:bind:bad/key>"), None);
    }

    #[test]
    fn open_marker_key_rejects_trailing_text() {
        assert_eq!(open_marker_key("// <gluon:bind:foo> extra"), None);
    }

    #[test]
    fn open_marker_key_requires_comment_prefix() {
        assert_eq!(open_marker_key("<gluon:bind:foo>"), None);
    }

    #[test]
    fn open_marker_key_rejects_close_marker() {
        assert_eq!(open_marker_key("// </gluon:bind:foo>"), None);
    }

    // line_is_close_marker cases.

    #[test]
    fn line_is_close_marker_matches_with_indent() {
        assert!(line_is_close_marker(
            "    // </gluon:bind:foo>",
            "</gluon:bind:foo>"
        ));
    }

    #[test]
    fn line_is_close_marker_rejects_trailing_text() {
        assert!(!line_is_close_marker(
            "// </gluon:bind:foo> extra",
            "</gluon:bind:foo>"
        ));
    }

    #[test]
    fn line_is_close_marker_rejects_open_marker() {
        assert!(!line_is_close_marker(
            "// <gluon:bind:foo>",
            "</gluon:bind:foo>"
        ));
    }

    #[test]
    fn insert_and_replace_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wiring.rs");
        std::fs::write(
            &path,
            r"use gluon::ContainerBuilder;

pub fn build_container(builder: ContainerBuilder) -> ContainerBuilder {
    let mut builder = builder;
    // <gluon:binds>
    // </gluon:binds>
    builder
}
",
        )
        .unwrap();

        insert_bind(&path, "usecase:list_users", "builder = builder.bind(U);").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("// <gluon:bind:usecase:list_users>"));
        assert!(content.contains("builder = builder.bind(U);"));

        // Re-insert under the same key must replace, not duplicate.
        insert_bind(&path, "usecase:list_users", "builder = builder.bind(V);").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content.matches("<gluon:bind:usecase:list_users>").count(),
            1,
            "duplicate block: {content}"
        );
        assert!(content.contains("builder = builder.bind(V);"));
        assert!(!content.contains("builder = builder.bind(U);"));
    }
}
