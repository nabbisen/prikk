//! Structural extraction for governed shell and workflow procedures.

use super::{Scan, basename, inert_head, publication, python_policy, rust_policy, scan_mode};

pub(super) fn allowed(tokens: &[String], index: usize, head: &str) -> bool {
    let tail = tokens.get(index + 1..).unwrap_or_default();
    match basename(head) {
        "python" | "python3" => python_policy(tail),
        "cargo" => {
            rust_policy(tail)
                || publication(tail).is_some()
                || tail
                    .split_first()
                    .is_some_and(|(command, arguments)| cargo(command, arguments))
        }
        "mdbook" => tail == ["build"],
        // DC-70, added per architect review (B1): `tar`, `rustc`, and `gh` can each execute
        // another program (tar via --to-command/-I, rustc via proc macros and build scripts,
        // gh as a general-purpose API client), so — unlike the inert set — they need an
        // exact-match entry, not blanket approval with any arguments.
        "tar" => tar(tail),
        "rustc" => {
            tail == [
                "-vV",
                ">>",
                "dist/prikk-x86_64-unknown-linux-gnu.build-info.txt",
            ] || tail
                == [
                    "-vV",
                    ">>",
                    "dist/prikk-aarch64-unknown-linux-gnu.build-info.txt",
                ]
        }
        "gh" => gh_release_create(tail),
        _ => inert_head(head),
    }
}

fn tar(tail: &[String]) -> bool {
    tail == [
        "-C",
        "stage",
        "-czf",
        "dist/prikk-x86_64-unknown-linux-gnu.tar.gz",
        "prikk",
        "LICENSE",
    ] || tail
        == [
            "-C",
            "stage",
            "-czf",
            "dist/prikk-aarch64-unknown-linux-gnu.tar.gz",
            "prikk",
            "LICENSE",
        ]
        // DC-71 B2 ruling: the CI fixture round-trip through tar, not the artifact zip, which
        // does not preserve empty directories — create on the fixture job, extract on the
        // conformance job. The lexer's sentence-trailing-period trim (normalize_token) reduces
        // tar's literal "." (current directory) argument to an empty, dropped token, so the
        // create form's tail is four elements, not five — confirmed against the tokenizer, not
        // assumed from the shell source.
        || tail == ["-czf", "fixture-repo.tar.gz", "-C", "fixture-repo"]
        || tail == ["-xzf", "fixture-repo.tar.gz", "-C", "fixture-repo"]
}

/// `gh release create $TAG <assets...> --repo nabbisen/prikk --title $TAG --notes-file <path>`.
/// The release tag cannot be enumerated in advance the way `cargo build`'s two targets are, so
/// this matches on shape instead: every other token is a fixed literal, and the two `$TAG`
/// positions (the release identifier and its title) must be the identical token, whatever it is.
fn gh_release_create(tail: &[String]) -> bool {
    let [
        action,
        subcommand,
        tag,
        assets @ ..,
        repo_flag,
        repo,
        title_flag,
        title,
        notes_flag,
        notes,
    ] = tail
    else {
        return false;
    };
    action == "release"
        && subcommand == "create"
        && assets
            == [
                "dist/*.tar.gz",
                "dist/*.tar.gz.sha256",
                "dist/*.build-info.txt",
            ]
        && repo_flag == "--repo"
        && repo == "nabbisen/prikk"
        && title_flag == "--title"
        && title == tag
        && notes_flag == "--notes-file"
        && notes == ".github/release-notes-template.md"
}

fn cargo(command: &str, arguments: &[String]) -> bool {
    match command {
        "fmt" => arguments == ["--check"] || arguments == ["--all", "--", "--check"],
        "test" => arguments == ["--workspace"] || arguments == ["--workspace", "--locked"],
        "clippy" => {
            arguments
                == [
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ]
        }
        // DC-71: CI must populate the cache for every target before the boundary check runs
        // `cargo metadata --locked --offline`. `fetch` downloads only; it cannot publish.
        "fetch" => arguments == ["--locked"],
        "check" => arguments == ["--workspace", "--all-targets", "--locked"],
        "build" => {
            arguments == ["--workspace", "--locked"]
                // DC-70: release.yml's two per-target release-binary builds, spelled out in
                // full rather than templated, so each is an exact, reviewable entry.
                || arguments
                    == [
                        "-p",
                        "prikk",
                        "--release",
                        "--target",
                        "x86_64-unknown-linux-gnu",
                        "--locked",
                    ]
                || arguments
                    == [
                        "-p",
                        "prikk",
                        "--release",
                        "--target",
                        "aarch64-unknown-linux-gnu",
                        "--locked",
                    ]
                // DC-71: ci.yml's read-only-fixture jobs build only the prikk binary, debug
                // profile, on whichever platform the job runs.
                || arguments == ["-p", "prikk", "--locked"]
        }
        "install" => {
            arguments
                == [
                    "mdbook",
                    "--no-default-features",
                    "--features",
                    "search",
                    "--vers",
                    "^0.5",
                    "--locked",
                ]
                // DC-77: docs.yml's mdbook-mermaid install, so Mermaid diagrams render as
                // pictures rather than code blocks. Exact match only — this arm accepts exactly
                // this vector, not `mdbook-mermaid` with any arguments (DC-70 B1 precedent).
                || arguments == ["mdbook-mermaid", "--vers", "^0.17", "--locked"]
        }
        _ => false,
    }
}

pub(crate) fn shell(text: &str) -> Scan {
    scan_mode(text, true)
}

pub(crate) fn yaml(text: &str) -> Scan {
    match yaml_scripts(text) {
        Ok(scripts) => {
            let mut result = Scan::default();
            for script in scripts {
                let scanned = scan_mode(&script, true);
                result.invocations.extend(scanned.invocations);
                result.errors.extend(scanned.errors);
            }
            result
        }
        Err(error) => Scan {
            invocations: Vec::new(),
            errors: vec![error],
        },
    }
}

fn yaml_scripts(text: &str) -> Result<Vec<String>, &'static str> {
    let lines: Vec<&str> = text.lines().collect();
    let mut scripts = Vec::new();
    let mut index = 0;
    while let Some(line) = lines.get(index) {
        let indent = indentation(line);
        let mapping = sequence_mapping(line.trim_start());
        let Some(value) = run_value(mapping)? else {
            index += 1;
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            if parent_key(&lines, index, indent) == Some("defaults") {
                index += 1;
                continue;
            }
            return Err("empty-yaml-run-scalar");
        }
        if value.starts_with('|') || value.starts_with('>') {
            let folded = value.starts_with('>');
            let (script, next) = block(&lines, index + 1, indent, folded);
            if script.is_empty() {
                return Err("empty-yaml-run-block");
            }
            scripts.push(script);
            index = next;
            continue;
        }
        scripts.push(scalar(value)?);
        index += 1;
    }
    Ok(scripts)
}

fn sequence_mapping(line: &str) -> &str {
    line.strip_prefix('-').map(str::trim_start).unwrap_or(line)
}

fn run_value(mapping: &str) -> Result<Option<&str>, &'static str> {
    if mapping.starts_with('{') {
        return flow_run_value(mapping);
    }
    let Ok((key, value)) = split_field(mapping) else {
        return Ok(None);
    };
    Ok((normalized_key(key) == "run").then_some(value))
}

fn flow_run_value(mapping: &str) -> Result<Option<&str>, &'static str> {
    let inner = mapping
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or("malformed-yaml-flow-mapping")?;
    let mut run = None;
    for field in flow_fields(inner)? {
        let (key, value) = split_field(field)?;
        if normalized_key(key) == "run" && run.replace(value.trim()).is_some() {
            return Err("duplicate-yaml-run-key");
        }
    }
    Ok(run)
}

fn normalized_key(key: &str) -> &str {
    key.trim().trim_matches(['\'', '"'])
}

fn parent_key<'a>(lines: &[&'a str], index: usize, child_indent: usize) -> Option<&'a str> {
    lines.get(..index)?.iter().rev().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || indentation(line) >= child_indent {
            return None;
        }
        split_field(sequence_mapping(trimmed))
            .ok()
            .map(|(key, _)| normalized_key(key))
    })
}

fn flow_fields(mapping: &str) -> Result<Vec<&str>, &'static str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0_u32;
    for (index, character) in mapping.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' | '{' => depth += 1,
            ']' | '}' => depth = depth.checked_sub(1).ok_or("malformed-yaml-flow-nesting")?,
            ',' if depth == 0 => {
                fields.push(mapping.get(start..index).unwrap_or_default());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() || escaped || depth != 0 {
        return Err("malformed-yaml-flow-mapping");
    }
    fields.push(mapping.get(start..).unwrap_or_default());
    Ok(fields)
}

fn split_field(field: &str) -> Result<(&str, &str), &'static str> {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0_u32;
    for (index, character) in field.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote == Some('"') && character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' | '{' => depth += 1,
            ']' | '}' => depth = depth.checked_sub(1).ok_or("malformed-yaml-flow-nesting")?,
            ':' if depth == 0 => {
                let key = field.get(..index).unwrap_or_default();
                let value = field
                    .get(index + character.len_utf8()..)
                    .unwrap_or_default();
                return Ok((key, value));
            }
            _ => {}
        }
    }
    Err("malformed-yaml-flow-field")
}

fn scalar(value: &str) -> Result<String, &'static str> {
    let value = value.trim_end_matches('}').trim_end();
    if value.starts_with(['|', '>']) {
        return Err("unsupported-inline-yaml-run");
    }
    if value.starts_with('"') || value.starts_with('\'') {
        return Err("unsupported-quoted-yaml-run-scalar");
    }
    Ok(value.to_owned())
}

fn block(lines: &[&str], mut index: usize, parent_indent: usize, folded: bool) -> (String, usize) {
    let mut output = String::new();
    while let Some(line) = lines.get(index) {
        if !line.trim().is_empty() && indentation(line) <= parent_indent {
            break;
        }
        if !output.is_empty() {
            output.push(if folded { ' ' } else { '\n' });
        }
        output.push_str(line.trim());
        index += 1;
    }
    (output, index)
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[cfg(test)]
#[path = "procedure/tests.rs"]
mod tests;
