//! Unit tests for the `client` submodule of `chat_stream`.
//!
//! Tests cover adapter kind mapping, URL normalization, and client construction.

#[cfg(test)]
mod tests {
    use crate::ai::agent_providers::chat_stream::client::{
        adapter_kind_for, build_client, build_user_agent_header, normalize_endpoint_url,
    };
    use crate::settings::AgentProviderApiType;
    use genai::adapter::AdapterKind;

    // -----------------------------------------------------------------------
    // adapter_kind_for
    // -----------------------------------------------------------------------

    #[test]
    fn test_adapter_kind_mapping_complete() {
        // Every AgentProviderApiType variant maps without panicking.
        let variants = [
            AgentProviderApiType::OpenAi,
            AgentProviderApiType::OpenAiResp,
            AgentProviderApiType::Gemini,
            AgentProviderApiType::Anthropic,
            AgentProviderApiType::Ollama,
            AgentProviderApiType::DeepSeek,
        ];
        let expected = [
            AdapterKind::OpenAI,
            AdapterKind::OpenAIResp,
            AdapterKind::Gemini,
            AdapterKind::Anthropic,
            AdapterKind::Ollama,
            AdapterKind::DeepSeek,
        ];
        for (variant, expect) in variants.iter().zip(expected.iter()) {
            assert_eq!(
                adapter_kind_for(*variant),
                *expect,
                "adapter_kind_for({variant:?}) mismatch"
            );
        }
    }

    // -----------------------------------------------------------------------
    // normalize_endpoint_url
    // -----------------------------------------------------------------------

    #[test]
    fn test_normalize_url_empty_fallback() {
        // Empty string uses the api_type's default base URL.
        let result = normalize_endpoint_url(AgentProviderApiType::OpenAi, "");
        assert_eq!(result, "https://api.openai.com/v1");

        let result = normalize_endpoint_url(AgentProviderApiType::Anthropic, "  ");
        assert_eq!(result, "https://api.anthropic.com/v1/");
    }

    #[test]
    fn test_normalize_url_bare_host() {
        // Bare host (no path) gets the default version path appended.
        let result = normalize_endpoint_url(
            AgentProviderApiType::OpenAi,
            "https://api.example.com",
        );
        assert_eq!(result, "https://api.example.com/v1");

        let result = normalize_endpoint_url(
            AgentProviderApiType::Anthropic,
            "https://proxy.anthropic.dev",
        );
        assert_eq!(result, "https://proxy.anthropic.dev/v1/");

        let result = normalize_endpoint_url(
            AgentProviderApiType::Ollama,
            "http://192.168.1.100:11434",
        );
        // Ollama default_base_url is "http://localhost:11434" with path "/"
        assert_eq!(result, "http://192.168.1.100:11434/");
    }

    #[test]
    fn test_normalize_url_custom_path_preserved() {
        // User-provided path is preserved, only trailing / ensured.
        let result = normalize_endpoint_url(
            AgentProviderApiType::OpenAi,
            "https://proxy.com/v2/chat",
        );
        assert_eq!(result, "https://proxy.com/v2/chat/");

        // Already has trailing /
        let result = normalize_endpoint_url(
            AgentProviderApiType::Anthropic,
            "https://proxy.com/custom/v1/",
        );
        assert_eq!(result, "https://proxy.com/custom/v1/");
    }

    #[test]
    fn test_normalize_url_all_known_types() {
        // Each api_type's default produces a valid URL (non-empty, starts with http).
        let types = [
            AgentProviderApiType::OpenAi,
            AgentProviderApiType::OpenAiResp,
            AgentProviderApiType::Gemini,
            AgentProviderApiType::Anthropic,
            AgentProviderApiType::Ollama,
            AgentProviderApiType::DeepSeek,
        ];
        for api_type in types {
            let url = normalize_endpoint_url(api_type, "");
            assert!(
                url.starts_with("http"),
                "normalize_endpoint_url({api_type:?}, \"\") = {url:?} does not start with http"
            );
            assert!(
                !url.is_empty(),
                "normalize_endpoint_url({api_type:?}, \"\") produced empty string"
            );
        }
    }

    // -----------------------------------------------------------------------
    // build_client
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_client_with_empty_api_key() {
        // Ollama local mode: api_key="" should not panic during client construction.
        let _client = build_client(
            AgentProviderApiType::Ollama,
            "http://localhost:11434".to_string(),
            String::new(),
        );
        // If we reach here without panic, the test passes.
    }
}
