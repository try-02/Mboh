//! Deterministic prompt/schema compaction for Jcode v0.99.1.
//!
//! Enabled with JCODE_TOKEN_OPTIMIZER=1. This module intentionally does not
//! remove tool properties, required fields, enums, types, or validation bounds.

use crate::message::ToolDefinition;
use crate::prompt::SplitSystemPrompt;
use serde_json::Value;

pub(super) fn apply(tools: &mut [ToolDefinition], prompt: &mut SplitSystemPrompt) {
    if !enabled() {
        return;
    }

    let prompt_before = prompt.static_part.len();
    let tools_before = ToolDefinition::aggregate_prompt_chars(tools);

    prompt.static_part = compact_static_prompt(&prompt.static_part);
    for tool in tools.iter_mut() {
        // Replace descriptions only for known built-ins. Preserve root descriptions
        // from MCP/custom tools verbatim; common boilerplate inside their schemas
        // can still be shortened without changing schema structure.
        if let Some(compact) = compact_builtin_tool_description(&tool.name) {
            tool.description = compact.to_string();
        }
        compact_schema_node(&mut tool.input_schema, None);
    }

    if report_enabled() {
        let prompt_after = prompt.static_part.len();
        let tools_after = ToolDefinition::aggregate_prompt_chars(tools);
        let before = prompt_before.saturating_add(tools_before);
        let after = prompt_after.saturating_add(tools_after);
        let saved = before.saturating_sub(after);
        crate::logging::info(&format!(
            "TOKEN_OPTIMIZER system_chars={prompt_before}->{prompt_after} tools_chars={tools_before}->{tools_after} saved_chars={saved} approx_tokens_saved={}",
            saved / 4
        ));
    }
}

fn enabled() -> bool {
    env_flag("JCODE_TOKEN_OPTIMIZER")
}

fn report_enabled() -> bool {
    env_flag("JCODE_TOKEN_OPTIMIZER_REPORT")
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

/// Compact only recognizable stock Jcode sections. User/project instructions,
/// skills, AGENTS.md, and unknown sections remain intact.
fn compact_static_prompt(input: &str) -> String {
    let mut prompt = input.to_string();
    prompt = rewrite_section(
        &prompt,
        "## Identity",
        "maximally helpful and proactive coding agent and assistant",
        "## Identity\nYou are Jcode, a proactive coding agent. Source: https://github.com/1jehuang/jcode.",
    );
    prompt = rewrite_section(
        &prompt,
        "## Autonomy and persistence",
        "Use todo tool extensively",
        "## Workflow\nComplete the user's goal autonomously; fix issues rather than only reporting them. Use todos for multi-step work when available. Be comprehensive and ask only when blocked. Pause before destructive or irreversible actions. Keep the final summary under five lines.",
    );
    prompt = rewrite_section(
        &prompt,
        "## Coding",
        "Edit files with `edit`, `replace`, `apply_patch`, or `write`",
        "## Coding\nCommit incrementally. Prefer swarm coordination over branches/worktrees when isolation is unnecessary. Use non-interactive commands. Edit with `edit`, `replace`, `apply_patch`, or `write`, not `sed`, `perl`, or Python scripts in Bash.",
    );
    prompt = rewrite_section(
        &prompt,
        "## Dont",
        "Don't use em dashes.",
        "## Don't\nAvoid em dashes and semicolons used as dashes. Do not deny academic help, reset passwords, or take actions likely to cause regret.",
    );
    normalize_markdown(&prompt)
}

/// Replace a whole heading section only when a stock-prompt fingerprint exists.
/// Unknown/custom sections are not semantically rewritten.
fn rewrite_section(input: &str, heading: &str, fingerprint: &str, replacement: &str) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let Some(start) = lines.iter().position(|line| line.trim() == heading) else {
        return input.to_string();
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| is_markdown_heading(line))
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    let section = lines[start..end].join("\n");
    if !section.contains(fingerprint) {
        return input.to_string();
    }

    let mut output = lines[..start].join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(replacement.trim());
    if end < lines.len() {
        output.push('\n');
        output.push_str(&lines[end..].join("\n"));
    }
    output
}

fn is_markdown_heading(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with('#') && line.chars().take_while(|ch| *ch == '#').count() <= 6
}

/// Collapse repeated blank lines outside fenced code blocks and trailing spaces.
fn normalize_markdown(input: &str) -> String {
    let mut output: Vec<String> = Vec::new();
    let mut in_fence = false;
    let mut previous_blank = false;

    for line in input.lines() {
        let fence_line = line.trim_start().starts_with("```");
        if in_fence {
            output.push(line.to_string());
            if fence_line {
                in_fence = false;
            }
            previous_blank = false;
            continue;
        }

        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            if !previous_blank {
                output.push(String::new());
            }
            previous_blank = true;
        } else {
            output.push(trimmed.to_string());
            previous_blank = false;
        }
        if fence_line {
            in_fence = true;
        }
    }

    output.join("\n").trim().to_string()
}

fn compact_builtin_tool_description(name: &str) -> Option<&'static str> {
    Some(match name {
        "agentgrep" => "Search files by content or name; supports grep, find, outline, and trace.",
        "apply_patch" => "Apply a multi-file patch or unified diff.",
        "bash" => "Run shell commands; prefer agentgrep or rg for repository searches.",
        "batch" => "Run independent tool calls in parallel; do not batch dependent calls.",
        "bg" => "Manage background tasks; prefer wait over polling.",
        "browser" => "Control a browser; use handoff for end-to-end tasks.",
        "calendar" => "List and manage calendar events.",
        "compile_remote" => "Compile source in Jcode's remote Linux sandbox; uploads files and spends credits.",
        "conversation_search" => "Search the current conversation history.",
        "edit" => "Edit files using exact text replacement.",
        "gmail" => "Search, read, draft, and manage Gmail.",
        "integration_tools" => "Find integrations for missing capabilities; inspect candidates before selecting. Off-catalog selections do not set up a provider.",
        "jcode_docs" => "Search or read bundled Jcode documentation.",
        "maintainer_feedback" => "Send feedback to Jcode maintainers; respects telemetry settings.",
        "mcp" => "Search/call MCP tools and manage servers; OAuth login is CLI-only.",
        "memory" => "Store, retrieve, search, and manage memory.",
        "open" => "Open or reveal a file, folder, or URL.",
        "panel" => "Show Markdown or PDF content in a side panel.",
        "read" => "Read text, image, or PDF files.",
        "schedule" => "Create, list, or cancel scheduled tasks.",
        "session_search" => "Search past sessions; tool noise is hidden by default.",
        "skill_manage" => "List, read, load, or reload installed skills.",
        "swarm" => "Coordinate implementation agents and manage their tasks.",
        "todo" => "Create and update todos and completion assessments.",
        "webfetch" => "Fetch a URL as text, Markdown, or HTML.",
        "websearch" => "Search the web.",
        "write" => "Write complete content to a file.",
        _ => return None,
    })
}

fn compact_description(property_name: Option<&str>, input: &str) -> String {
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();

    if property_name == Some("intent")
        && (lower.contains("why this call is being made")
            || lower.contains("purpose shown in the ui")
            || lower.contains("label shown in the ui"))
    {
        return "Reason for this call.".to_string();
    }
    if property_name == Some("accept_large_output")
        && (lower.contains("truncated") || lower.contains("withheld result"))
    {
        return "Accept the stated token cost to return a truncated result.".to_string();
    }

    normalized
        .replace(
            "Required short label shown in the UI: why this call is being made.",
            "Reason for this call.",
        )
        .replace("Brief purpose shown in the UI.", "Reason for this call.")
        .replace(
            "Re-run accepting the stated token cost of a withheld result.",
            "Accept the stated token cost to return a truncated result.",
        )
        .replace(
            "Set true to accept a truncated large result.",
            "Accept truncated output.",
        )
}

fn compact_schema_node(value: &mut Value, property_name: Option<&str>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(description)) = object.get_mut("description") {
                *description = compact_description(property_name, description);
            }

            // Keep every property and validation keyword. Only descriptions are
            // compacted; required/type/enum/bounds/additionalProperties stay as-is.
            for (key, child) in object.iter_mut() {
                if key == "description" {
                    continue;
                }
                if key == "properties" {
                    if let Value::Object(properties) = child {
                        for (name, property_schema) in properties.iter_mut() {
                            compact_schema_node(property_schema, Some(name));
                        }
                    }
                } else {
                    compact_schema_node(child, property_name);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                compact_schema_node(item, property_name);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compacts_only_stock_prompt_sections() {
        let input = "## Identity\n\nYou are Jcode. You are a maximally helpful and proactive coding agent and assistant.\nJcode is open source: <https://github.com/1jehuang/jcode>\n\n## Autonomy and persistence\n\nUse todo tool extensively\nHave autonomy. Persist to completing a task.\nRequesting input from user is a blocking action. Use this sparsely.\nUser response summary should be under 5 lines\nHesitate for destructive or non-reversible actions.\n\n# Mermaid\n\nRender fenced `mermaid` blocks inline.";
        let output = compact_static_prompt(input);
        assert!(output.contains("You are Jcode, a proactive coding agent."));
        assert!(output.contains("Use todos for multi-step work when available."));
        assert!(output.contains("# Mermaid"));
        assert!(output.len() < input.len());
    }

    #[test]
    fn leaves_custom_prompt_sections_semantically_unchanged() {
        let input = "## Identity\nYou are Jcode, running in Termux.\n\n## Workflow\nInspect before editing.";
        assert_eq!(compact_static_prompt(input), input);
    }

    #[test]
    fn compacts_repeated_tool_description_boilerplate() {
        assert_eq!(
            compact_description(
                Some("intent"),
                "Required short label shown in the UI: why this call is being made."
            ),
            "Reason for this call."
        );
        assert!(compact_builtin_tool_description("bash")
            .unwrap()
            .contains("repository searches"));
    }

    #[test]
    fn schema_structure_and_validation_are_preserved() {
        let mut schema = serde_json::json!({
            "type": "object",
            "required": ["action", "intent"],
            "properties": {
                "intent": {"type": "string", "description": "Required short label shown in the UI: why this call is being made."},
                "mode": {"type": "string", "enum": ["grep", "find"], "description": "Mode selector."},
                "limit": {"type": "integer", "minimum": 1, "maximum": 10}
            }
        });
        let before_required = schema["required"].clone();
        let before_mode = schema["properties"]["mode"]["enum"].clone();
        compact_schema_node(&mut schema, None);
        assert_eq!(schema["required"], before_required);
        assert_eq!(schema["properties"]["mode"]["enum"], before_mode);
        assert_eq!(schema["properties"]["intent"]["description"], "Reason for this call.");
        assert_eq!(schema["properties"]["limit"]["minimum"], 1);
        assert_eq!(schema["properties"]["limit"]["maximum"], 10);
    }

    #[test]
    fn prompt_compaction_is_idempotent() {
        let input = "## Identity\n\nYou are Jcode. You are a maximally helpful and proactive coding agent and assistant.\n\n## Coding\n\nEdit files with `edit`, `replace`, `apply_patch`, or `write`, not sed, perl, or Python scripts in bash.";
        let once = compact_static_prompt(input);
        let twice = compact_static_prompt(&once);
        assert_eq!(once, twice);
    }
}
