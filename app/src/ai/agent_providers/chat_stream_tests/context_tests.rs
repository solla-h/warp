//! Unit tests for the `context` submodule of `chat_stream`.
//!
//! These test the XML escaping helpers, `latest_input_context`, and
//! `render_ssh_session_block`. Written TDD-first: the functions are currently
//! private in `chat_stream.rs` and will be extracted to
//! `chat_stream/context.rs` as `pub(super)`.

#[cfg(test)]
mod tests {
    // After extraction, the import path will be:
    //   use super::super::chat_stream::context::{xml_text, xml_attr, ...};
    // For now we use the flat re-export that the test wiring provides.
    use crate::ai::agent_providers::chat_stream::context::{
        latest_input_context, render_ssh_session_block, xml_attr, xml_text,
    };

    use std::sync::Arc;

    use crate::ai::agent::{AIAgentContext, AIAgentInput, UserQueryMode};
    use crate::ai::blocklist::SessionContext;

    fn make_context_with_git(head: &str) -> Arc<[AIAgentContext]> {
        Arc::from(vec![AIAgentContext::Git {
            head: head.to_string(),
            branch: None,
        }])
    }

    // -----------------------------------------------------------------------
    // xml_text
    // -----------------------------------------------------------------------

    #[test]
    fn test_xml_text_escapes_special_chars() {
        // & must become &amp;
        assert_eq!(xml_text("a & b"), "a &amp; b");
        // < must become &lt;
        assert_eq!(xml_text("a < b"), "a &lt; b");
        // > must become &gt;
        assert_eq!(xml_text("a > b"), "a &gt; b");
        // Combined
        assert_eq!(xml_text("<tag>&</tag>"), "&lt;tag&gt;&amp;&lt;/tag&gt;");
    }

    #[test]
    fn test_xml_text_preserves_whitespace() {
        assert_eq!(xml_text("a\nb\tc\rd"), "a\nb\tc\rd");
    }

    #[test]
    fn test_xml_text_strips_control_chars() {
        // BEL (0x07) and ESC (0x1b) should become spaces
        assert_eq!(xml_text("a\x07b\x1bc"), "a b c");
        // DEL (0x7f) should become a space
        assert_eq!(xml_text("a\x7fb"), "a b");
    }

    // -----------------------------------------------------------------------
    // xml_attr
    // -----------------------------------------------------------------------

    #[test]
    fn test_xml_attr_escapes_special_chars() {
        // Same escapes as xml_text
        assert_eq!(xml_attr("a & b"), "a &amp; b");
        assert_eq!(xml_attr("a < b"), "a &lt; b");
        assert_eq!(xml_attr("a > b"), "a &gt; b");
        // Additionally, double-quote must become &quot;
        assert_eq!(xml_attr(r#"say "hello""#), "say &quot;hello&quot;");
    }

    #[test]
    fn test_xml_attr_quotes_and_entities_combined() {
        assert_eq!(
            xml_attr(r#"<a href="x&y">"#),
            "&lt;a href=&quot;x&amp;y&quot;&gt;"
        );
    }

    // -----------------------------------------------------------------------
    // latest_input_context
    // -----------------------------------------------------------------------

    #[test]
    fn test_latest_input_context_returns_last() {
        let ctx_early = make_context_with_git("aaa111");
        let ctx_late = make_context_with_git("bbb222");

        let inputs = vec![
            AIAgentInput::UserQuery {
                query: "first".to_string(),
                context: ctx_early,
                static_query_type: None,
                referenced_attachments: Default::default(),
                user_query_mode: UserQueryMode::Normal,
                running_command: None,
                intended_agent: None,
            },
            AIAgentInput::UserQuery {
                query: "second".to_string(),
                context: ctx_late.clone(),
                static_query_type: None,
                referenced_attachments: Default::default(),
                user_query_mode: UserQueryMode::Normal,
                running_command: None,
                intended_agent: None,
            },
        ];

        let result = latest_input_context(&inputs);
        // Should return the context from the LAST input that has context
        assert_eq!(result.len(), 1);
        match &result[0] {
            AIAgentContext::Git { head, .. } => {
                assert_eq!(head, "bbb222");
            }
            _ => panic!("Expected Git variant"),
        }
    }

    #[test]
    fn test_latest_input_context_none_when_no_context() {
        // Empty input slice
        let result = latest_input_context(&[]);
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // render_ssh_session_block
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_ssh_session_block_format() {
        // When is_legacy_ssh() returns false, should return None
        let non_ssh = SessionContext::new_for_test();
        assert_eq!(render_ssh_session_block(&non_ssh), None);
    }
}
