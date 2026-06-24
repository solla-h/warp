# Code Review: BYOP Agent Providers (ported from slim-zap)

审查日期: 2026-06-24
审查范围: app/src/ai/agent_providers/, app/src/ai/byop_compaction/, app/src/ai/byop_readiness/

---

## Critical (需要立即修复)

### C1: 完整请求体以 INFO 级别写入日志 — 隐私泄露
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:3289-3290`
- **代码**: `log::info!("[byop-diag] full_request_json={diag_body_json}");`
- **影响**: 每次 BYOP 请求都将完整对话历史(含用户消息、工具结果、system prompt)写入 INFO 日志。任何有日志访问权限的人可看到全部用户对话内容。
- **修复**: 改为 `log::debug!` 或完全移除

### C2: 中文硬编码注入到 LLM 消息中
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:1582-1586`
- **代码**: `"请按下面的技能...指引执行任务"` 和 `"用户进一步指令:"`
- **影响**: InvokeSkill 分支将中文字符串注入给上游 LLM。英文模型(GPT/Claude)收到中文指令会行为退化。
- **修复**: 替换为英文 "Follow the skill instructions below:" / "User's additional instructions:"

---

## High (高优先级)

### H1: custom-ep-N ID 不稳定 — 历史对话指向错误 provider
- **文件**: `app/src/ai/agent_providers/mod.rs:194-220`
- **问题**: `custom_endpoints_as_providers` 用 enumerate 索引生成 provider ID (`custom-ep-0`, `custom-ep-1`)。如果用户在 Settings 中删除或插入一个 endpoint，所有后续 ID 漂移，已保存对话中的 `byop:custom-ep-2:model` 会指向错误的 provider。
- **修复**: 基于 URL 的 hash 或给 CustomEndpoint 加 UUID 字段

### H2: fetch_openai_compatible_models 无超时
- **文件**: `app/src/ai/agent_providers/openai_compatible.rs`
- **问题**: HTTP GET `/models` 请求无 timeout，可被阻塞的服务器无限挂起
- **修复**: 加 `tokio::time::timeout(Duration::from_secs(15), ...)`

### H3: cancellation_rx 被丢弃 — 用户取消无效
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:3208`
- **问题**: `cancellation_rx: _cancellation_rx` 绑定后从未使用。用户取消操作时流不会中断。
- **修复**: 在 streaming loop 中用 `select!` 监听 cancellation

### H4: 多个 AIAgentInput 变体被静默忽略
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:1636-1642`
- **被忽略的**: `AutoCodeDiffQuery`, `CreateNewProject`, `CodeReview`, `CloneRepository`, `FetchReviewComments`
- **影响**: 如果这些输入到达 BYOP 路径，产生空消息，模型行为不可预测
- **修复**: 返回明确错误或在 upstream gate 这些变体

---

## Medium (中等优先级)

### M1: MCP 工具结果用 Debug 格式输出给 LLM
- **文件**: `app/src/ai/agent_providers/tools/mcp.rs:327,336`
- **代码**: `format!("{:?}", s)` 输出 Rust Debug repr 作为工具结果
- **影响**: LLM 看到 `CallMcpToolSuccess { content: [Content { ... }] }` 而非干净 JSON，浪费 token 且难解析
- **修复**: 实现 proper JSON serialization for `rmcp::Content`

### M2: 全局 lint 压制掩盖真实问题
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:1`
- **代码**: `#![allow(dead_code, unused_imports, unused_variables)]`
- **影响**: 所有死代码、未使用导入、未使用变量的警告被静默。真实问题无法被编译器发现。
- **修复**: 移除全局 allow，逐个修复具体 warning

### M3: EXA_API_KEY 仅从环境变量读取
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:4586`
- **代码**: `std::env::var("EXA_API_KEY").ok()`
- **影响**: 无 Settings UI 集成，用户不知道需要设置这个环境变量
- **修复**: 集成到 AgentProviderSecrets 或 Settings UI

### M4: 工具名 fallback 用 Debug 格式字符串拼接
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:2581-2588`
- **代码**: `format!("{other:?}").split('(').next().unwrap_or("UnknownVariant")`
- **影响**: 依赖 Rust Debug 输出格式，任何 enum 重命名都会产生错误工具名
- **修复**: 为所有 Tool 变体添加 explicit match arm

### M5: prompt_renderer Jinja2 模板注入风险
- **文件**: `app/src/ai/agent_providers/prompt_renderer.rs`
- **问题**: 用户内容直接插入 minijinja 模板上下文，如果内容包含 `{{` 语法可能导致模板注入
- **修复**: 对用户内容进行转义或使用 `|safe` 以外的注入方式

### M6: Gemini thinking signatures 被静默丢弃
- **文件**: `app/src/ai/agent_providers/chat_stream.rs:1884`
- **问题**: ThoughtSignatureChunk 不持久化，Gemini 2.5 Pro 多轮推理质量降级
- **修复**: 持久化 thought_signatures 或在 prompt 中回传

---

## Low (低优先级 / 代码卫生)

### L1: 死代码 summarization_overflow (line 1232-1238)
- 与 `is_summarization_request` 完全重复，计算后立即丢弃

### L2: web_runtime.rs SSRF 防护中每次请求新建 Client
- 应复用 pre-built Client 以利用连接池

### L3: htmd 库用 catch_unwind 防 panic
- `web_runtime.rs:416` — 说明 htmd 库有已知 panic bug，考虑替换

### L4: warp_internal_* fallback 工具名包含 "warp" 品牌
- `chat_stream.rs:2587` — 应改为 `marb_internal_*`

### L5: 多处 `zerx-lab/warp #25` issue tracker 引用
- `chat_stream.rs:251,307,5103` — 应移除或替换为内部 tracker

### L6: byop_readiness 中的 user-facing 错误消息仍引用 "Marb" 
- `byop_readiness/mod.rs:12` — 确认品牌名正确

---

## 整体评估

| 模块 | 评分 | 核心问题 |
|------|------|---------|
| tools/ (22 files) | 8.5/10 | MCP Debug 格式是唯一功能性缺陷 |
| chat_stream.rs (7314 lines) | 6.5/10 | 隐私日志、中文注入、死代码、取消无效 |
| byop_compaction/ | 7/10 | 状态同步风险、依赖未 port 的 RepairState |
| byop_readiness/ | 7.5/10 | 逻辑严谨但部分功能未完成 |
| mod.rs + secrets + llm_id | 8/10 | ID 稳定性是唯一高优问题 |

**总结**: 架构设计合理，核心逻辑严谨（无 unwrap 在用户输入上），但 port 过程中遗留了多处适配问题。最严重的是隐私日志和中文注入 — 这两个应在下次 release 前修复。
