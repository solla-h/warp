//! ChatOptions building, tools array construction, and prompt caching.
//!
//! Extracted from `mod.rs` to isolate the "what options/tools/cache params do we
//! send to the upstream model" concern from the streaming execution logic.

use genai::chat::{CacheControl, ChatMessage, ChatOptions, ChatRole, Tool as GenaiTool};
use serde_json::json;

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentInput, UserQueryMode};
use crate::ai::agent_providers::tools;
use crate::settings::AgentProviderApiType;

// ---------------------------------------------------------------------------
// Prompt caching — Anthropic per-message cache_control
// ---------------------------------------------------------------------------

/// 1:1 移植自 opencode `provider/transform.ts::applyCaching` 的 Anthropic 分支:
/// 给 first 2 个 system message + last 2 个 non-system message 打 cache 标记。
///
/// genai Anthropic adapter 在 `into_anthropic_request_parts` 内把
/// `MessageOptions::cache_control` 落到该 message 最后一个 content part 上,
/// 行为与 opencode 给 lastContent.providerOptions.anthropic.cacheControl 一致。
///
/// **TTL 选择(修订自 P0-4,对齐 Zed `crates/anthropic/src/completion.rs:219-274`)**:
/// 静态前缀 system 用 1h;会话尾部 last 2 non-system 用 5m;同时 `build_chat_request`
/// 在 Anthropic 路径给 tools 末尾打 1h(genai Tool struct 已加 cache_control 字段)。
///
/// **混合策略动机**:
/// - 旧策略"全部 1h"无法和外部 5m breakpoint 共存。
/// - 新策略:长 TTL 全部前置(tools / system 1h),短 TTL 落在序列尾(messages 5m)。
///
/// **TTL 排序约束**:Anthropic API 要求长 TTL 的 breakpoint 必须排在短 TTL 之前。
/// 本函数把 system 标 1h、非 system 末尾标 5m,顺序天然 system(1h) → messages(5m) 合规。
pub(crate) fn apply_caching_anthropic(messages: &mut Vec<ChatMessage>) {
    let n = messages.len();
    if n == 0 {
        return;
    }
    let mut tag = vec![false; n];

    // first 2 system messages
    let mut sys_seen = 0;
    for (i, m) in messages.iter().enumerate() {
        if matches!(m.role, ChatRole::System) {
            tag[i] = true;
            sys_seen += 1;
            if sys_seen >= 2 {
                break;
            }
        }
    }
    // last 2 non-system messages
    let mut tail_seen = 0;
    for (i, m) in messages.iter().enumerate().rev() {
        if !matches!(m.role, ChatRole::System) {
            tag[i] = true;
            tail_seen += 1;
            if tail_seen >= 2 {
                break;
            }
        }
    }

    let original = std::mem::take(messages);
    *messages = original
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            if tag[i] {
                let ttl = if matches!(m.role, ChatRole::System) {
                    CacheControl::Ephemeral1h
                } else {
                    CacheControl::Ephemeral5m
                };
                m.with_options(ttl)
            } else {
                m
            }
        })
        .collect();
}

// ---------------------------------------------------------------------------
// Plan mode detection
// ---------------------------------------------------------------------------

/// 本轮 input 是否含 `/plan` 触发的 `UserQueryMode::Plan`。
pub(crate) fn is_plan_mode_turn(input: &[AIAgentInput]) -> bool {
    input.iter().any(|i| {
        matches!(
            i,
            AIAgentInput::UserQuery {
                user_query_mode: UserQueryMode::Plan,
                ..
            }
        )
    })
}

// ---------------------------------------------------------------------------
// Tools array construction
// ---------------------------------------------------------------------------

/// Plan Mode 下硬过滤的写/执行类内置工具名。
pub(crate) const PLAN_MODE_BLOCKED_TOOLS: &[&str] = &[
    "run_shell_command",
    "apply_file_diffs",
    "write_to_long_running_shell_command",
    "open_code_review",
    "transfer_shell_command_control_to_user",
    "suggest_prompt",
];

/// 列出本轮真正会喂给上游模型的 tool name(内置 REGISTRY + 当前 MCP 工具),
/// 与 `build_tools_array` 共享同一套 gating(LRC / `web_search_enabled` /
/// `suggest_new_conversation` / `plan_mode`)。供 `prompt_renderer` 注入到
/// system prompt,让模板按实际可用列表动态渲染,不再硬编码白/黑名单。
pub fn available_tool_names(params: &RequestParams) -> Vec<String> {
    let is_lrc = params.lrc_command_id.is_some();
    let web_enabled = params.web_search_enabled;
    let plan_mode = is_plan_mode_turn(&params.input);
    let mut names: Vec<String> = tools::REGISTRY
        .iter()
        .filter(|t| {
            if is_lrc && t.name == "run_shell_command" {
                return false;
            }
            if !web_enabled
                && (t.name == tools::webfetch::TOOL_NAME || t.name == tools::websearch::TOOL_NAME)
            {
                return false;
            }
            if t.name == "suggest_new_conversation" {
                return false;
            }
            if plan_mode && PLAN_MODE_BLOCKED_TOOLS.contains(&t.name) {
                return false;
            }
            true
        })
        .map(|t| t.name.to_owned())
        .collect();
    if let Some(ctx) = params.mcp_context.as_ref() {
        for (name, _description, _parameters) in tools::mcp::build_mcp_tool_defs(ctx) {
            names.push(name);
        }
    }
    names
}

pub(crate) fn build_tools_array(params: &RequestParams) -> Vec<GenaiTool> {
    let is_lrc = params.lrc_command_id.is_some();
    let web_enabled = params.web_search_enabled;
    let plan_mode = is_plan_mode_turn(&params.input);
    let current_year = chrono::Local::now().format("%Y").to_string();
    let mut out: Vec<GenaiTool> = tools::REGISTRY
        .iter()
        .filter(|t| {
            if is_lrc && t.name == "run_shell_command" {
                return false;
            }
            if !web_enabled
                && (t.name == tools::webfetch::TOOL_NAME || t.name == tools::websearch::TOOL_NAME)
            {
                return false;
            }
            if t.name == "suggest_new_conversation" {
                return false;
            }
            if plan_mode && PLAN_MODE_BLOCKED_TOOLS.contains(&t.name) {
                return false;
            }
            true
        })
        .map(|t| {
            let description = if t.description.contains("{{year}}") {
                t.description.replace("{{year}}", &current_year)
            } else {
                t.description.to_owned()
            };
            GenaiTool::new(t.name)
                .with_description(description)
                .with_schema((t.parameters)())
        })
        .collect();

    if let Some(ctx) = params.mcp_context.as_ref() {
        for (name, description, parameters) in tools::mcp::build_mcp_tool_defs(ctx) {
            out.push(
                GenaiTool::new(name)
                    .with_description(description)
                    .with_schema(parameters),
            );
        }
    }
    if is_lrc {
        log::info!(
            "[byop] LRC tag-in: tools array filtered (removed run_shell_command), \
             total tools={}",
            out.len()
        );
    }
    if plan_mode {
        log::info!(
            "[byop] Plan Mode: tools array filtered (removed write/exec tools: {:?}), \
             total tools={}",
            PLAN_MODE_BLOCKED_TOOLS,
            out.len()
        );
    }
    out
}

// ---------------------------------------------------------------------------
// DashScope thinking gate
// ---------------------------------------------------------------------------

/// 判定是否给 DashScope(阿里云百炼,OpenAI 兼容路径)注入 `enable_thinking: true`。
///
/// 命中条件(全部满足):
/// 1. `api_type == OpenAi`
/// 2. `effort_setting != Off`
/// 3. base_url 含 `dashscope.aliyuncs.com` / `dashscope.cn` / `dashscope-intl.aliyuncs.com`
/// 4. model_id 不含 `kimi-k2-thinking`
/// 5. model_id 命中 reasoning 子串白名单
pub(crate) fn dashscope_needs_enable_thinking(
    api_type: AgentProviderApiType,
    base_url: &str,
    model_id: &str,
    effort_setting: crate::settings::ReasoningEffortSetting,
) -> bool {
    if !matches!(api_type, AgentProviderApiType::OpenAi) {
        return false;
    }
    if matches!(effort_setting, crate::settings::ReasoningEffortSetting::Off) {
        return false;
    }
    let url = base_url.to_ascii_lowercase();
    let is_dashscope = url.contains("dashscope.aliyuncs.com")
        || url.contains("dashscope.cn")
        || url.contains("dashscope-intl.aliyuncs.com");
    if !is_dashscope {
        return false;
    }
    let id = model_id.to_ascii_lowercase();
    if id.contains("kimi-k2-thinking") {
        return false;
    }
    id.contains("qwen3")
        || id.contains("qwq")
        || id.contains("deepseek-r1")
        || id.contains("kimi-k2.5")
        || id.contains("kimi-k2-")
        || id.contains("qwen-plus")
}

// ---------------------------------------------------------------------------
// ChatOptions construction
// ---------------------------------------------------------------------------

/// 按 base_url 反推上游 provider,仅用于决定是否下发 `prompt_cache_key`。
fn opencode_compatible_cache_provider(base_url: &str) -> bool {
    let u = base_url.to_ascii_lowercase();
    u.contains("api.openai.com")
        || u.contains(".openai.azure.com")
        || u.contains("openrouter.ai/api")
        || u.contains("api.venice.ai/api")
        || u.contains("opencode.ai/zen")
}

pub(crate) fn build_chat_options(
    api_type: AgentProviderApiType,
    base_url: &str,
    model_id: &str,
    effort_setting: crate::settings::ReasoningEffortSetting,
    extra_headers: Vec<(String, String)>,
    conversation_id: Option<&str>,
) -> ChatOptions {
    let mut opts = ChatOptions::default()
        .with_capture_content(true)
        .with_capture_tool_calls(true)
        .with_capture_reasoning_content(true)
        .with_capture_usage(true)
        .with_normalize_reasoning_content(true);

    // Prompt caching: only whitelisted providers get prompt_cache_key.
    if matches!(
        api_type,
        AgentProviderApiType::OpenAi | AgentProviderApiType::OpenAiResp
    ) && opencode_compatible_cache_provider(base_url)
    {
        if let Some(cid) = conversation_id {
            if !cid.is_empty() {
                opts = opts.with_prompt_cache_key(cid.to_owned());
            }
        }
    }

    // Reasoning effort dispatch per provider.
    use crate::settings::ReasoningEffortSetting as RE;
    match (api_type, effort_setting) {
        // Auto: no params sent
        (_, RE::Auto) => {}

        // Anthropic + Off: skip reasoning_effort entirely
        (AgentProviderApiType::Anthropic, RE::Off) => {
            log::info!(
                "[byop] Anthropic Off → skip reasoning_effort (model={model_id}); \
                 no thinking field sent"
            );
        }

        // Gemini + Off: skip thinkingConfig
        (AgentProviderApiType::Gemini, RE::Off) => {
            log::info!(
                "[byop] Gemini Off → skip reasoning_effort (model={model_id}); \
                 no thinkingConfig sent"
            );
        }

        // DeepSeek + Off: explicit disabled
        (AgentProviderApiType::DeepSeek, RE::Off) => {
            log::info!(
                "[byop] DeepSeek Off → extra_body thinking.type=disabled (model={model_id})"
            );
            opts = opts.with_extra_body(json!({"thinking": {"type": "disabled"}}));
        }

        // All others: capability-gated reasoning_effort injection
        _ => {
            if let Some(effort) = effort_setting.to_genai() {
                if crate::ai::agent_providers::reasoning::model_supports_reasoning(
                    api_type, model_id,
                ) {
                    log::info!(
                        "[byop] reasoning_effort injected: model={model_id} setting={effort_setting:?}"
                    );
                    opts = opts.with_reasoning_effort(effort);
                } else {
                    log::info!(
                        "[byop] reasoning_effort SKIPPED: model={model_id} not in capability list \
                         (api_type={api_type:?} setting={effort_setting:?}); request sent without thinking params"
                    );
                }
            }
        }
    }

    // DashScope enable_thinking injection.
    if dashscope_needs_enable_thinking(api_type, base_url, model_id, effort_setting) {
        log::info!(
            "[byop] DashScope reasoning model → extra_body enable_thinking=true \
             (model={model_id} setting={effort_setting:?})"
        );
        opts = opts.with_extra_body(json!({"enable_thinking": true}));
    }
    if !extra_headers.is_empty() {
        opts = opts.with_extra_headers(extra_headers);
    }

    opts
}
