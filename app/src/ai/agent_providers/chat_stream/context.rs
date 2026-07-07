//! Context rendering utilities for chat_stream.
//!
//! Responsible for:
//! - XML text/attribute escaping (safe for embedding in system prompts)
//! - Extracting the latest AIAgentContext from input
//! - Rendering LRC (Live Running Command) context blocks
//! - Rendering SSH session context blocks

use crate::ai::agent::api::RequestParams;
use crate::ai::agent::{AIAgentContext, AIAgentInput, RunningCommand};

/// Extract the contexts from the most recent input that carries one.
pub(crate) fn latest_input_context(input: &[AIAgentInput]) -> &[AIAgentContext] {
    for i in input.iter().rev() {
        if let Some(ctx) = i.context() {
            return ctx;
        }
    }
    &[]
}

/// Render `<attached_running_command>` XML block for LRC tag-in scenarios.
///
/// Prepended to user message so the model sees the current PTY state
/// (command, grid contents, alt-screen status) and can correctly choose
/// `write_to_long_running_shell_command` tool.
pub(crate) fn render_running_command_context(rc: &RunningCommand) -> String {
    format!(
        "<attached_running_command command_id=\"{}\" is_alt_screen_active=\"{}\">\n  \
         <command>{}</command>\n  \
         <snapshot>\n{}\n  </snapshot>\n  \
         <instructions>This command is already running in the user's terminal. \
         Use `read_shell_command_output` with this command_id to inspect it, and \
         `write_to_long_running_shell_command` with this command_id to operate the program \
         through its PTY (in raw mode, use tokens like `<ESC>` and `<ENTER>` for control \
         keys). This command_id is valid even if the process was started by the user \
         rather than by run_shell_command. Do NOT spawn a new shell to control the same TUI.\
         </instructions>\n\
         </attached_running_command>",
        xml_attr(rc.block_id.as_str()),
        rc.is_alt_screen_active,
        xml_text(&rc.command),
        xml_text(&rc.grid_contents),
    )
}

/// Fallback version: only has command_id (no full RunningCommand snapshot).
pub(crate) fn render_running_command_id_context(command_id: &str) -> String {
    format!(
        "<attached_running_command command_id=\"{}\">\n  \
         <instructions>This command is already running in the user's terminal. \
         Use `read_shell_command_output` with this command_id to inspect it, and \
         `write_to_long_running_shell_command` with this command_id to operate the program \
         through its PTY. Do NOT spawn a new shell to control the same TUI.</instructions>\n\
         </attached_running_command>",
        xml_attr(command_id),
    )
}

/// Render LRC context from RequestParams (tries full RunningCommand first, then just ID).
pub(crate) fn render_lrc_request_context(params: &RequestParams) -> Option<String> {
    params
        .lrc_running_command
        .as_ref()
        .map(render_running_command_context)
        .or_else(|| {
            params
                .lrc_command_id
                .as_deref()
                .map(render_running_command_id_context)
        })
}

/// Render SSH session context block for legacy SSH sessions.
///
/// Returns `None` if the session is not a legacy SSH session.
pub(crate) fn render_ssh_session_block(
    session_context: &crate::ai::blocklist::SessionContext,
) -> Option<String> {
    if !session_context.is_legacy_ssh() {
        return None;
    }
    let info = session_context.ssh_connection_info();
    let host = info
        .and_then(|i| i.host.as_deref())
        .map(xml_attr)
        .unwrap_or_else(|| "unknown".to_owned());
    let port = info
        .and_then(|i| i.port.as_deref())
        .map(xml_attr)
        .unwrap_or_else(|| "22".to_owned());

    Some(format!(
        "\n\n<ssh_session host=\"{host}\" port=\"{port}\">\n  \
         <fact>The active terminal PTY is currently inside an SSH session opened by the user from their local machine. \
         All shell commands you run via `run_shell_command` execute on the REMOTE host, not on the local client.</fact>\n  \
         <warning>The [Environment] block (OS / shell / working directory) above describes the LOCAL client and may not match the remote host. \
         If you need precise remote info, probe it directly (e.g. `uname -a`, `cat /etc/os-release`, `pwd`).</warning>\n  \
         <rules>\n    \
         - Run commands DIRECTLY (e.g. `uname -a`, `ls /`). Do NOT prepend `ssh {host} ...` — that opens a NESTED ssh session inside the current one.\n    \
         - Treat the working directory and home directory shown above with skepticism; they may reflect the local client.\n    \
         - When LRC tag-in mode is active (an `<attached_running_command>` block is present), prefer `write_to_long_running_shell_command` with that command_id to inject keystrokes into this same remote PTY. Spawning a new shell would create a separate local-side ssh client, not interact with the remote process the user is watching.\n  \
         </rules>\n\
         </ssh_session>"
    ))
}

/// XML-escape text content, stripping illegal control characters.
///
/// - `\n`, `\r`, `\t` are preserved (JSON-legal)
/// - Other chars < 0x20 and DEL (0x7f) become spaces
/// - `&`, `<`, `>` become XML entities
pub(crate) fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' | '\r' | '\t' => out.push(c),
            c if (c as u32) < 0x20 => out.push(' '),
            '\u{7f}' => out.push(' '),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// XML-escape for attribute values (additionally escapes `"`).
pub(crate) fn xml_attr(s: &str) -> String {
    xml_text(s).replace('"', "&quot;")
}

