# Master TODO — 三轮深度审查综合计划

## 依赖关系图

```
                    ┌─────────────────────────────┐
                    │  T0: 运行时验证 (你手动)       │
                    │  - /agent hello 测试          │
                    │  - ConPTY DLL 验证            │
                    └─────────────┬───────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
    ┌─────▼─────┐          ┌─────▼─────┐          ┌─────▼─────┐
    │ T1: OTel   │          │ T2: Secrets│          │ T3: AWS   │
    │ optional   │          │ optional   │          │ feature   │
    │ (独立)     │          │ (独立)     │          │ (独立)    │
    └─────┬─────┘          └─────┬─────┘          └─────┬─────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │  T4: Workspace Exclude       │
                    │  9 unused crates (独立)       │
                    └─────────────┬───────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
    ┌─────▼─────┐          ┌─────▼─────┐          ┌─────▼─────┐
    │ T5: Types  │          │ T6: GraphQL│          │ T7: UI    │
    │ extraction │          │ optional   │          │ cleanup   │
    │ (T4之后)   │          │ (T5之后)   │          │ (独立)    │
    └─────┬─────┘          └─────┬─────┘          └─────┬─────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  │
                    ┌─────────────▼───────────────┐
                    │  T8: cloud_objects optional  │
                    │  (最终目标，依赖 T5+T6)      │
                    └─────────────────────────────┘
```

## 并行分组

```
═══════════════════════════════════════════════════════════════
 并行组 A (无依赖，立即可执行):    T1 + T2 + T3 + T4 + T7
═══════════════════════════════════════════════════════════════
 串行组 B (依赖组 A 完成):         T5 → T6 → T8
═══════════════════════════════════════════════════════════════
```

---

## T0: 运行时验证 [你手动] — 阻塞一切后续

### 前置条件: 无
### 验证项:
- [ ] 启动 `warp-oss.exe` + 附带 DLLs (conpty.dll 等)
- [ ] 确认 **无 onboarding** 直接进入终端
- [ ] 确认 **无需登录** 可以使用终端
- [ ] 配置 BYOP provider (Settings > AI > Custom Endpoints)
- [ ] 测试 `/agent hello` 收到 LLM 回复
- [ ] 测试工具调用: `/agent 读取 README.md`

### 已修复:
- onboarding skip: runtime `Channel::Oss` check (`fec3f9cb`)
- login bypass: driver.rs + mod.rs + root_view.rs (`fec3f9cb`)
- 8处 AI login gates: harness/admin/llms (`e339378e`)
- FreeAvailableModels no-op: llms.rs (`34a24eb3`)

---

## T1: OpenTelemetry Optional [可并行] — 最高 ROI

### 前置条件: 无
### 预估: 30 分钟
### 效果: 减 14 个 crate (含 prost transitive)

### 步骤:
- [ ] `app/Cargo.toml`: 5 个 otel dep 加 `optional = true`
- [ ] 新增 feature: `otel = ["dep:opentelemetry", "dep:opentelemetry-http", "dep:opentelemetry-otlp", "dep:opentelemetry_sdk", "dep:tracing-opentelemetry"]`
- [ ] 加入 `default` feature list
- [ ] `app/src/tracing.rs`: wrap `native::init()` 调用 in `#[cfg(feature = "otel")]`
- [ ] `app/src/tracing.rs`: wrap `start_auth_refresh()` in `#[cfg(feature = "otel")]`  
- [ ] `app/src/tracing/native.rs`: 加 `#![cfg(feature = "otel")]` 模块级 gate
- [ ] `app/src/tracing/cloud_agent_auth.rs`: 加 `#![cfg(feature = "otel")]` 模块级 gate

### 验证:
- [ ] `cargo check --bin warp-oss` (0 errors)
- [ ] `cargo check --bin warp-oss --features local-only` (0 errors)
- [ ] `cargo tree --bin warp-oss --no-default-features --features local-only 2>/dev/null | grep opentelemetry` (无输出)

---

## T2: warp_managed_secrets Optional [可并行]

### 前置条件: 无
### 预估: 1 小时
### 效果: 减 tink×3 + hpke + prost + rand 0.9 (~15 crate)

### 当前状态: 90% 已 optional (init gated)，仅剩 6 处 type-only 引用

### 步骤:
- [ ] `app/Cargo.toml`: `warp_managed_secrets = { workspace = true, optional = true }`
- [ ] 加入 `cloud` feature: `"dep:warp_managed_secrets"`
- [ ] Gate 6 个 type-only 引用 in:
  - `app/src/terminal/view/ambient_agent/auth_secret_*.rs` (3 files) — `SecretOwner`
  - `app/src/ai/auth_secret_types.rs` — `ManagedSecretValue`
  - `app/src/ai/mcp/mod_tests.rs` — test
  - `app/src/ai/mcp/templatable_installation.rs` — type ref
- [ ] `crates/warp_server_auth/Cargo.toml`: 确认 `managed_secrets` feature 已存在

### 验证:
- [ ] 双模式 0 errors
- [ ] `cargo tree` 无 tink/hpke (when local-only + no cloud)

---

## T3: AWS Bedrock Feature [可并行]

### 前置条件: 无
### 预估: 2 小时
### 效果: 减 22 个 AWS crate (对不需要 Bedrock 的用户)

### 步骤:
- [ ] `app/Cargo.toml`: 4 个 AWS dep 加 `optional = true`
- [ ] 新增 feature: `aws-bedrock = ["dep:aws-config", "dep:aws-credential-types", "dep:aws-sdk-sts", "dep:aws-types"]`
- [ ] 加入 `default` feature list
- [ ] Gate `app/src/ai/bedrock_credentials.rs`: `#![cfg(feature = "aws-bedrock")]`
- [ ] Gate `app/src/ai/agent_sdk/driver/bedrock_credentials.rs`: `#![cfg(feature = "aws-bedrock")]`
- [ ] Gate `app/src/ai/agent_sdk/driver/cloud_provider/aws.rs`: `#![cfg(feature = "aws-bedrock")]`
- [ ] `app/src/ai/aws_credentials.rs`: gate OIDC leaf functions (`refresh_aws_credentials_oidc`, `sts_client`, `aws_role_session_name`) with `#[cfg(feature = "aws-bedrock")]`
- [ ] Provide no-op stub for `AwsCredentialRefresher` when feature disabled

### 验证:
- [ ] `cargo check --bin warp-oss` (0 errors — default includes aws-bedrock)
- [ ] `cargo check --bin warp-oss --no-default-features --features local-only` (0 errors)
- [ ] 本地 BYO Bedrock 用户仍可用 (default build)

---

## T4: Workspace Exclude 9 Unused Crates [可并行]

### 前置条件: 无 (这些 crate 已证实未被 local build 使用)
### 预估: 15 分钟
### 效果: IDE 索引加速，`cargo` 命令更快

### 步骤:
- [ ] `Cargo.toml` workspace `exclude` 列表添加:
  ```toml
  exclude = [
    "crates/serve-wasm",        # 已有
    "crates/managed_secrets_wasm",
    "crates/warp_web_event_bus",
    "crates/virtual_fs",
    "crates/integration",
  ]
  ```
  注: 只排除 WASM/test crate；cloud crate 暂不排除（仍被 transitive 引用）

### 验证:
- [ ] `cargo check --bin warp-oss` (0 errors)

---

## T5: ServerTimestamp 提取到 warp_types [依赖 T4]

### 前置条件: T4 完成
### 预估: 2 小时
### 效果: 22 处 import 不再需要 warp_graphql

### 步骤:
- [ ] 创建 `crates/warp_types/` (仅 chrono + serde dep)
- [ ] 移动 `ServerTimestamp` 和 `Uint32` scalar 定义
- [ ] warp_graphql re-export from warp_types (backward compat)
- [ ] 22 处 consumer 改为 `use warp_types::ServerTimestamp`
- [ ] Workspace 注册新 crate

### 验证:
- [ ] 双模式 0 errors
- [ ] `cargo tree -p warp_types` 无 cynic dep

---

## T6: warp_graphql App 层 Optional [依赖 T5]

### 前置条件: T5 完成 (scalar 已提取，减少 22 处引用)
### 预估: 1-2 天
### 效果: 减 cynic + graphql-ws-client + prost 系列 (~30 crate)

### 步骤:
- [ ] `app/Cargo.toml`: `warp_graphql = { workspace = true, optional = true }`
- [ ] 加入 `cloud` feature
- [ ] Gate 73 个 Category B (shared type) 引用:
  - billing types (12 处): stub enum
  - ai types (8 处): stub `AgentTaskState`/`PlatformErrorCode`
  - object_permissions (15 处): stub `AccessLevel`/`OwnerType`
  - managed_secrets types (7 处): already should be gone from T2
  - generic_string_object (5 处): stub `GenericStringObjectFormat`
- [ ] Gate 105 个 Category A (RPC) 引用: 全在 server/ (已 runtime gate)
- [ ] Gate 18 个 Category C (scalar): 已由 T5 处理

### 验证:
- [ ] 双模式 0 errors
- [ ] `cargo tree --bin warp-oss --no-default-features --features local-only | grep cynic` (0)

---

## T7: UI 清理 — 隐藏云页面 [可并行]

### 前置条件: 无
### 预估: 2 小时
### 效果: UX 整洁 — local 用户不看到无意义的云功能

### 步骤:
- [ ] Settings 中隐藏 6 个云页面 (Channel::Oss 时):
  - billing_and_usage_page
  - environments_page  
  - platform_page (API keys)
  - referrals_page
  - teams_page
  - warp_drive_page
- [ ] Gate onboarding::init() 调用 (Channel::Oss 跳过)
- [ ] Gate remote_server/ init (Channel::Oss 跳过，24 files)
- [ ] Gate agent_sdk/ 10 个纯云文件 (OAuth/schedule/integration)

### 验证:
- [ ] 双模式 0 errors
- [ ] 启动时无 onboarding
- [ ] Settings 中无云页面

---

## T8: cloud_objects Optional [最终目标，依赖 T5+T6]

### 前置条件: T5 (types extracted) + T6 (warp_graphql optional)
### 预估: 1 周+
### 效果: 完全断开 102 文件的云依赖链

### 步骤:
- [ ] B1: 从 cloud_objects 内联 UserUid (已验证可行)
- [ ] 让 cloud_objects 变 optional in app
- [ ] 102 处引用: 提供 stub types 或 cfg gate
- [ ] cloud_object_client, cloud_object_models 随之 optional

### 验证:
- [ ] `cargo tree --bin warp-oss --no-default-features --features local-only` 无任何 cloud crate
- [ ] 编译时间对比: 预估减 30-40%
- [ ] 二进制体积对比: 预估 325MB → ~220MB

---

## 量化预期收益

| 完成阶段 | Dep tree 减少 | Binary 估算 | 编译加速 |
|----------|-------------|-------------|---------|
| 当前 (T0) | 0 | 326 MB | baseline |
| T1 (otel) | -14 crate | -10 MB | -5% |
| T2 (secrets) | -15 crate | -8 MB | -5% |
| T3 (AWS) | -22 crate | -15 MB | -8% |
| T4 (exclude) | workspace 加速 | - | -3% |
| T5+T6 (graphql) | -30 crate | -20 MB | -10% |
| T8 (cloud_objects) | -50+ crate | -50 MB | -15% |
| **合计** | **~130 crate** | **~220 MB** | **~45%** |
