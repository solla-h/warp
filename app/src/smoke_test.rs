use std::time::Duration;

use ::ai::api_keys::ApiKeyManager;
use ::settings::Setting;
use futures::StreamExt;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatRole, ChatStreamEvent};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use warpui::{AppContext, SingletonEntity};

use crate::settings::{AISettings, AgentProviderApiType};

const SMOKE_TEST_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn run_smoke_test(ctx: &mut AppContext) {
    eprintln!("[smoke-test] starting BYOP stream verification...");

    let (base_url, api_key, model_id, api_type) = match resolve_byop_endpoint(ctx) {
        Some(info) => info,
        None => {
            eprintln!("[smoke-test] FAIL: no BYOP endpoint configured");
            std::process::exit(1);
        }
    };

    eprintln!(
        "[smoke-test] endpoint={} model={} api_type={:?}",
        base_url, model_id, api_type
    );

    ctx.background_executor().spawn(async move {
        match tokio::time::timeout(SMOKE_TEST_TIMEOUT, execute_stream(&base_url, &api_key, &model_id, api_type)).await {
            Ok(Ok(chunk_count)) => {
                log::info!(
                    "[byop] stream stats: start=1 chunks={chunk_count}"
                );
                eprintln!(
                    "[smoke-test] PASS: stream completed with {chunk_count} chunks"
                );
                std::process::exit(0);
            }
            Ok(Err(e)) => {
                eprintln!("[smoke-test] FAIL: stream error: {e:#}");
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("[smoke-test] FAIL: timeout after {}s", SMOKE_TEST_TIMEOUT.as_secs());
                std::process::exit(1);
            }
        }
    })
    .detach();
}

fn resolve_byop_endpoint(
    ctx: &AppContext,
) -> Option<(String, String, String, AgentProviderApiType)> {
    let ai_settings = AISettings::as_ref(ctx);
    let providers = ai_settings.agent_providers.value().clone();

    for provider in &providers {
        if !provider.base_url.trim().is_empty() && !provider.models.is_empty() {
            let api_key = crate::ai::agent_providers::AgentProviderSecrets::as_ref(ctx)
                .get(&provider.id)
                .map(str::to_owned)
                .unwrap_or_default();
            let model_id = provider.models[0].id.clone();
            return Some((
                provider.base_url.clone(),
                api_key,
                model_id,
                provider.api_type,
            ));
        }
    }

    let keys = ApiKeyManager::as_ref(ctx).keys();
    for ep in &keys.custom_endpoints {
        if ep.url.trim().is_empty() || ep.models.is_empty() {
            continue;
        }
        let api_type = crate::ai::agent_providers::parse_api_type(&ep.api_type, &ep.models);
        let model_id = ep.models[0].name.clone();
        return Some((ep.url.clone(), ep.api_key.clone(), model_id, api_type));
    }

    None
}

async fn execute_stream(
    base_url: &str,
    api_key: &str,
    model_id: &str,
    api_type: AgentProviderApiType,
) -> anyhow::Result<usize> {
    use crate::ai::agent_providers::chat_stream::adapter_kind_for;

    let adapter_kind = adapter_kind_for(api_type);
    let model_iden = ModelIden::new(adapter_kind, model_id.to_string());

    let endpoint_url = if base_url.ends_with('/') { base_url.to_string() } else { format!("{}/", base_url) };
    let key = api_key.to_string();
    let resolver = ServiceTargetResolver::from_resolver_fn(
        move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let ServiceTarget { model, .. } = service_target;
            Ok(ServiceTarget {
                endpoint: Endpoint::from_owned(endpoint_url.clone()),
                auth: AuthData::from_single(key.clone()),
                model,
            })
        },
    );

    let client = Client::builder()
        .with_service_target_resolver(resolver)
        .build();

    let mut chat_req = ChatRequest::default();
    chat_req = chat_req.append_message(ChatMessage::new(ChatRole::User, "ping"));

    let chat_opts = ChatOptions::default();
    let stream_resp = client
        .exec_chat_stream(&model_iden, chat_req, Some(&chat_opts))
        .await
        .map_err(|e| anyhow::anyhow!("exec_chat_stream failed: {e}"))?;

    let mut chunk_count: usize = 0;
    let mut stream = stream_resp.stream;
    while let Some(event) = stream.next().await {
        match event {
            Ok(ChatStreamEvent::Chunk(chunk)) => {
                if !chunk.content.is_empty() {
                    chunk_count += 1;
                }
            }
            Ok(_) => {}
            Err(e) => {
                return Err(anyhow::anyhow!("stream chunk error: {e}"));
            }
        }
    }

    if chunk_count == 0 {
        return Err(anyhow::anyhow!("stream returned 0 content chunks"));
    }

    Ok(chunk_count)
}
