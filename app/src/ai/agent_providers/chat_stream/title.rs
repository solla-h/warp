//! Title generation via BYOP oneshot completion.

use crate::settings::AgentProviderApiType;

/// BYOP configuration needed for title generation. May differ from the main
/// request provider if the user selected a separate title model.
pub struct TitleGenInput {
    pub base_url: String,
    pub api_key: String,
    pub model_id: String,
    pub api_type: AgentProviderApiType,
    pub reasoning_effort: crate::settings::ReasoningEffortSetting,
}

/// Generate a conversation title using the user's BYOP provider.
///
/// Delegates to `oneshot::byop_oneshot_streaming_completion`; this function
/// assembles the prompt and sanitizes the output.
///
/// ## Prompt design
///
/// - **system**: see `prompts/tasks/title_system.md`
/// - **user**: wraps `user_query` in `<user>...</user>` tags
/// - **temperature**: 0.5
pub(crate) async fn generate_title_via_byop(
    tg: &TitleGenInput,
    user_query: &str,
) -> Result<Option<String>, anyhow::Error> {
    let cfg = super::super::oneshot::OneshotConfig {
        base_url: tg.base_url.clone(),
        api_key: tg.api_key.clone(),
        model_id: tg.model_id.clone(),
        api_type: tg.api_type,
        reasoning_effort: tg.reasoning_effort,
    };
    let system = include_str!("../prompts/tasks/title_system.md");
    let user_prompt = format!(
        "Generate a title for this conversation:\n<user>{}</user>",
        user_query
    );
    let opts = super::super::oneshot::OneshotOptions {
        max_chars: Some(1000),
        temperature: Some(0.5),
        ..Default::default()
    };
    let raw = super::super::oneshot::byop_oneshot_completion(&cfg, system, &user_prompt, &opts).await?;
    Ok(sanitize_title(&raw))
}

/// Sanitize raw title text. Returns `None` for empty result (upstream skips emit).
///
/// Processing order:
/// 1. Strip `<think>...</think>` / `<reasoning>...</reasoning>` blocks.
/// 2. Take first non-empty line.
/// 3. Strip `Title:` / related prefixes (case-insensitive).
/// 4. Strip leading/trailing quotes.
/// 5. Strip trailing punctuation.
/// 6. Truncate to 50 chars (by char, protecting CJK), appending `...` if needed.
pub(crate) fn sanitize_title(raw: &str) -> Option<String> {
    // 1. Strip reasoning tags (may be multiple, DOTALL mode).
    let mut s = raw.to_owned();
    for tag in &["think", "reasoning", "thought", "scratchpad"] {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);
        while let (Some(start), Some(end_rel)) =
            (s.find(&open), s.find(&close).map(|e| e + close.len()))
        {
            if end_rel <= start {
                break;
            }
            s.replace_range(start..end_rel, "");
        }
    }

    // 2. Take first non-empty line.
    let first_line = s
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_owned();
    let mut s = first_line;

    // 3. Strip prefixes (loop for double-prefix like "Title: \u{6807}\u{9898}: foo").
    let prefixes = [
        "title:",
        "subject:",
        "thread:",
        "\u{6807}\u{9898}:",
        "\u{6807}\u{9898}\u{ff1a}",
        "\u{4e3b}\u{9898}:",
        "\u{4e3b}\u{9898}\u{ff1a}",
    ];
    loop {
        let lower = s.to_lowercase();
        let mut stripped = false;
        for p in &prefixes {
            if lower.starts_with(p) {
                s = s[p.len()..].trim_start().to_owned();
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    // 4. Strip leading/trailing quotes (CJK and ASCII).
    let quotes = ['"', '\'', '`', '\u{201c}', '\u{201d}', '\u{2018}', '\u{2019}', '\u{300a}', '\u{300b}', '\u{300c}', '\u{300d}'];
    while let Some(c) = s.chars().next() {
        if quotes.contains(&c) {
            s.remove(0);
        } else {
            break;
        }
    }
    while let Some(c) = s.chars().last() {
        if quotes.contains(&c) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    // 5. Strip trailing punctuation.
    while let Some(c) = s.chars().last() {
        if matches!(
            c,
            '.' | '\u{3002}' | '!' | '\u{ff01}' | '?' | '\u{ff1f}' | ',' | '\u{ff0c}' | ';' | '\u{ff1b}' | ':' | '\u{ff1a}'
        ) {
            let new_len = s.len() - c.len_utf8();
            s.truncate(new_len);
        } else {
            break;
        }
    }

    let s = s.trim().to_owned();
    if s.is_empty() {
        return None;
    }

    // 6. Truncate to 50 chars (by char, protecting CJK).
    const MAX_CHARS: usize = 50;
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > MAX_CHARS {
        let mut truncated: String = chars.iter().take(MAX_CHARS - 1).collect();
        truncated.push('\u{2026}');
        Some(truncated)
    } else {
        Some(s)
    }
}
