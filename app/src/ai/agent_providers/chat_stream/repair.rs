//! History gap repair: converts `AcceptedRepair`-authorized gaps into outbound
//! structured `ToolResponse` placeholders for the API request body.

use std::collections::{HashMap, HashSet};

use genai::chat::{ChatMessage, ChatRole, ToolResponse};
use serde_json::json;

use ai::agent::convert::ConvertToAPITypeError;
use crate::ai::byop_readiness::{
    AcceptedRepair, BlockedByopReadinessError, ReadinessCategory, RepairSource, ToolCallKey,
};

use super::serialization::OutboundAssistantToolGroup;

pub(crate) const REPAIR_PLACEHOLDER_NOTE: &str =
    "tool result was unavailable in repaired conversation history";

pub(crate) fn is_placeholder_tool_response_content(content: &str) -> bool {
    use serde_json::Value;

    if content == "(tool \u{6267}\u{884c}\u{7ed3}\u{679c}\u{672a}\u{4fdd}\u{7559})" {
        return true;
    }

    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(content) else {
        return false;
    };

    object.len() == 3
        && object.get("status").and_then(Value::as_str) == Some("unavailable")
        && matches!(
            object.get("reason").and_then(Value::as_str),
            Some("forked_history_repair" | "restored_legacy_history_repair")
        )
        && object.get("note").and_then(Value::as_str) == Some(REPAIR_PLACEHOLDER_NOTE)
}

pub(crate) fn should_replace_tool_response(existing: &ToolResponse, candidate: &ToolResponse) -> bool {
    is_placeholder_tool_response_content(&existing.content)
        || !is_placeholder_tool_response_content(&candidate.content)
}

fn insert_preferred_tool_response(
    responses_by_call_id: &mut HashMap<String, ToolResponse>,
    response: &ToolResponse,
) {
    let should_replace = match responses_by_call_id.get(&response.call_id) {
        None => true,
        Some(existing) => should_replace_tool_response(existing, response),
    };
    if should_replace {
        responses_by_call_id.insert(response.call_id.clone(), response.clone());
    }
}

pub(crate) fn repair_placeholder_content(source: RepairSource) -> String {
    json!({
        "status": "unavailable",
        "reason": source.placeholder_reason(),
        "note": REPAIR_PLACEHOLDER_NOTE,
    })
    .to_string()
}

/// Only runs after the serializer has determined `AcceptedHistoryRepair`:
/// converts RepairRecord-authorized history gaps into outbound-only structured
/// ToolResponse placeholders. Normal missing/duplicate/orphan/out-of-order gaps
/// are blocked at readiness validation; this function never generates placeholders
/// for normal flow.
pub(crate) fn repair_tool_call_pairs_for_accepted_history_gaps(
    messages: &mut Vec<ChatMessage>,
    repairs: &[AcceptedRepair],
    outbound_tool_groups: &[OutboundAssistantToolGroup],
) -> Result<(), ConvertToAPITypeError> {
    if repairs.is_empty() {
        return Ok(());
    }

    let repair_by_key: HashMap<ToolCallKey, &AcceptedRepair> = repairs
        .iter()
        .map(|repair| (repair.tool_call.key.clone(), repair))
        .collect();
    let group_by_message_index: HashMap<usize, &OutboundAssistantToolGroup> = outbound_tool_groups
        .iter()
        .map(|group| (group.message_index, group))
        .collect();
    let mut call_id_counts: HashMap<String, usize> = HashMap::new();
    for group in outbound_tool_groups {
        for key in &group.tool_call_keys {
            *call_id_counts.entry(key.tool_call_id.clone()).or_default() += 1;
        }
    }
    let mut placeholders_inserted: Vec<String> = Vec::new();
    let mut orphan_call_ids: Vec<String> = Vec::new();
    let mut missing_without_repair: Vec<String> = Vec::new();

    let original = std::mem::take(messages);
    let mut late_responses_by_unique_call_id: HashMap<String, ToolResponse> = HashMap::new();
    let mut late_response_call_ids: HashSet<String> = HashSet::new();
    for (idx, msg) in original.iter().enumerate() {
        if msg.role != ChatRole::Tool {
            continue;
        }

        let is_adjacent_to_group =
            idx > 0 && group_by_message_index.contains_key(&(idx.saturating_sub(1)));
        if is_adjacent_to_group {
            continue;
        }

        for resp in msg.content.tool_responses() {
            if call_id_counts.get(&resp.call_id) == Some(&1) {
                insert_preferred_tool_response(&mut late_responses_by_unique_call_id, resp);
                late_response_call_ids.insert(resp.call_id.clone());
            }
        }
    }

    let mut rewritten: Vec<ChatMessage> = Vec::with_capacity(original.len());
    let mut idx = 0;
    while idx < original.len() {
        let msg = original[idx].clone();
        if msg.role == ChatRole::Tool {
            orphan_call_ids.extend(
                msg.content
                    .tool_responses()
                    .iter()
                    .filter(|response| !late_response_call_ids.contains(&response.call_id))
                    .map(|response| response.call_id.clone()),
            );
            idx += 1;
            continue;
        }

        let Some(group) = group_by_message_index.get(&idx).copied() else {
            rewritten.push(msg);
            idx += 1;
            continue;
        };

        rewritten.push(msg);
        idx += 1;

        let mut responses_by_call_id: HashMap<String, ToolResponse> = HashMap::new();
        while idx < original.len() && original[idx].role == ChatRole::Tool {
            for resp in original[idx].content.tool_responses() {
                insert_preferred_tool_response(&mut responses_by_call_id, resp);
            }
            idx += 1;
        }

        let mut bundled: Vec<ToolResponse> = Vec::new();
        for key in &group.tool_call_keys {
            let cid = &key.tool_call_id;
            let mut response = responses_by_call_id.remove(cid);
            if call_id_counts.get(cid) == Some(&1) {
                if let Some(late_response) = late_responses_by_unique_call_id.remove(cid) {
                    response = match response {
                        Some(existing)
                            if !should_replace_tool_response(&existing, &late_response) =>
                        {
                            Some(existing)
                        }
                        _ => Some(late_response),
                    };
                }
            }

            match response {
                Some(resp) => bundled.push(resp),
                None => {
                    if let Some(repair) = repair_by_key.get(key) {
                        placeholders_inserted.push(cid.clone());
                        bundled.push(ToolResponse::new(
                            cid.clone(),
                            repair_placeholder_content(repair.record.source),
                        ));
                    } else {
                        missing_without_repair.push(cid.clone());
                    }
                }
            }
        }

        if !bundled.is_empty() {
            rewritten.push(ChatMessage::from(bundled));
        }

        if !responses_by_call_id.is_empty() {
            orphan_call_ids.extend(responses_by_call_id.into_keys());
        }
    }

    *messages = rewritten;

    if !orphan_call_ids.is_empty() {
        log::warn!(
            "[byop-diag] accepted_history_repair: \u{4e22}\u{5f03} {} \u{4e2a}\u{5b64}\u{513f} ToolResponse: \
             orphan_call_ids={:?}",
            orphan_call_ids.len(),
            orphan_call_ids
        );
    }
    if !placeholders_inserted.is_empty() {
        log::info!(
            "[byop-diag] accepted_history_repair: \u{7ed9} {} \u{4e2a} ToolCall \u{8865} repair placeholder \
             ToolResponse: missing_call_ids={:?}",
            placeholders_inserted.len(),
            placeholders_inserted
        );
    }
    if !missing_without_repair.is_empty() {
        log::error!(
            "[byop-diag] accepted_history_repair: readiness \u{672a}\u{6388}\u{6743}\u{7684}\u{7f3a}\u{5931} ToolResponse: \
             missing_call_ids={:?}",
            missing_without_repair
        );
        return Err(ConvertToAPITypeError::Other(
            BlockedByopReadinessError::new(ReadinessCategory::MissingResultWithoutRepairSource)
                .into(),
        ));
    }
    Ok(())
}
