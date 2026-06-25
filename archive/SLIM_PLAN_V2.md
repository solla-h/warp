# 瘦身执行计划 V2 — 基于 #5-#9 Crate 深度审查

## 审查结论汇总

### #5: warp_server_client (2377 subtree deps)
- **状态**: 已 optional (behind `cloud` feature)
- **结论**: 整个 crate 除 `skip_login` path 外全是云网络调用
- **仅被 app 使用** (10 引用, 4 文件)
- **可提取**: `AuthEvent` + `AgentIdentity` → `warp_auth_types`
- **核心价值为零**: local build 不需要此 crate

### #6: cloud_objects (1668 subtree deps)
- **状态**: 已 optional (behind `cloud` feature)
- **结论**: `ids` 子模块 (2500+ 引用) 完全可提取到 `warp_types`
- **单一 UI 方法阻塞**: `CloudObjectStatuses::render_icon()` 是唯一拉入 warpui_core + pathfinder 的原因
- **drive/auth 子模块**: 仅 7+2 引用，可移出
- **关键发现**: ObjectIdType, ObjectType, JsonObjectType 等都是零依赖纯枚举

### #7: cloud_object_persistence (1651 subtree deps)
- **状态**: 已 optional (behind `cloud` feature)
- **结论**: **100% 可移除** — 每个函数都是纯云同步(object_metadata/permissions/refresh)
- **零本地价值**: 25 个函数全部服务于 server sync lifecycle
- **证据**: encode/decode_permissions, upsert/delete_cloud_object, mark_as_synced 等

### #8: warp_server_auth (1610 subtree deps)
- **状态**: 已 optional (behind `cloud` feature)
- **结论**: 5 个纯值类型可提取到 `warp_types`
- **可提取**: UserUid, UserMetadata, AnonymousUserType, PrincipalType, PersonalObjectLimits
- **本地 daemon 接口**: `apply_remote_server_auth_context` 是 local daemon 的关键边界
- **AuthState.is_logged_in()**: 多处已加 Oss bypass，本地路径正常

### #9: http_client (核心层，本地必需)
- **状态**: 非 optional（BYOP 需要 SSE）
- **结论**: IAP 模块和 Warp headers 是纯云代码，可 feature-gate
- **可 gate**: `iap.rs` 整体, `add_warp_http_headers()`, `is_warp_server_origin()`
- **本地必需**: Client, RequestBuilder, Response, SSE, bearer_auth, json, proto
- **建议 feature**: `warp-cloud = ["warp-headers", "iap"]`

---

## 额外关键发现

### genai crate 影响极小
- 仅引入 2 个新 crate (value-ext, eventsource-stream)
- reqwest/tokio/serde 全部与 workspace 共享，零版本冲突

### Binary 体积可优化
- 当前 711MB (debug sections 占 327MB = 46%)
- `strip = "debuginfo"` → ~384MB
- `strip = "symbols"` → ~310MB  
- `lto = "thin"` 额外减 10-20%

### 测试覆盖 Gap
- 20 个 Channel::Oss 运行时条件分支中 0 个被单元测试覆盖
- AI 可用性、Agent SDK auth bypass 完全无测试
- 零 Oss 集成测试

---

## 执行计划

### Phase 1: 类型提取 (解锁后续所有工作)

#### T9: 提取 ID 类型到 warp_types
**前置**: T5 已完成 (warp_types crate 存在)
**工作量**: 1-2 天
**效果**: 2500+ 引用不再依赖 cloud_objects

- [ ] 移动 ClientId, SyncId, ServerId, ObjectUid 到 warp_types
- [ ] 移动 HashableId, ToServerId traits 到 warp_types
- [ ] 移动 server_id_traits! macro 到 warp_types
- [ ] 移动 ObjectIdType enum 到 warp_types
- [ ] cloud_objects re-export from warp_types (backward compat)
- [ ] 验证: cargo check --bin warp-oss (0 errors)
- [ ] 验证: cargo test -p warp_types (ID tests pass)

#### T10: 提取 Auth 值类型到 warp_types
**前置**: T9
**工作量**: 0.5 天

- [ ] 移动 UserUid (sans cynic::Id impl) 到 warp_types
- [ ] 移动 UserMetadata, AnonymousUserType, PrincipalType, PersonalObjectLimits
- [ ] warp_server_auth re-export from warp_types
- [ ] cynic::Id conversion 留在 warp_graphql bridge
- [ ] 验证: 双模式 0 errors

### Phase 2: 真正的编译时隔离 (Approach B)

#### T11: http_client feature split
**前置**: 无
**工作量**: 0.5 天

- [ ] 添加 features: `warp-cloud = ["warp-headers", "iap"]`
- [ ] Gate `mod iap` behind `#[cfg(feature = "iap")]`
- [ ] Gate `add_warp_http_headers` behind `#[cfg(feature = "warp-headers")]`
- [ ] `include_warp_http_headers()` 返回 false when feature off
- [ ] app 的 http_client dep 添加 `features = ["warp-cloud"]` (default on)
- [ ] 验证: cargo check -p http_client --no-default-features (no warp_core dep)

#### T12: 196 文件 warp_graphql import gating (最大工作)
**前置**: T9 (IDs 不再需要 cloud_objects), T10 (auth types 不再需要 warp_server_auth)
**工作量**: 3-5 天
**策略**: 按 module 分批, 从最独立的开始

- [ ] Phase A: server/ 模块 (已 runtime gate) — 整体 `#[cfg(feature = "cloud")]` mod 声明
- [ ] Phase B: cloud_object/ 模块 — 同上
- [ ] Phase C: workspaces/ 模块 — 同上
- [ ] Phase D: drive/ 模块 — 同上
- [ ] Phase E: 散落的 73 个 shared type imports — stub or re-export
- [ ] 验证: `cargo check --bin warp-oss --no-default-features --features local-only` (0 errors)

#### T13: Binary 体积优化
**前置**: 无 (独立)
**工作量**: 15 分钟

- [ ] Cargo.toml `[profile.release]` 添加 `strip = "debuginfo"`
- [ ] 验证: release build size < 400MB
- [ ] 可选: `lto = "thin"` (编译慢但更小)

### Phase 3: 测试补全

#### T14: Channel::Oss 单元测试
**前置**: 无
**工作量**: 1 天

- [ ] 测试 is_any_ai_enabled() 对 Channel::Oss 返回 true
- [ ] 测试 is_byo_api_key_enabled() 对 Channel::Oss 返回 true
- [ ] 测试 is_custom_inference_enabled() 对 Channel::Oss 返回 true
- [ ] 测试 has_completed_local_onboarding() 对 Channel::Oss 返回 true
- [ ] 测试 AgentDriver login bypass for Channel::Oss
- [ ] 测试 LLMPreferences refresh bypass for Channel::Oss
- [ ] 验证: cargo test -p warp --lib -- oss (all pass)

#### T15: BYOP 集成测试
**前置**: T14
**工作量**: 1 天

- [ ] 测试 BYOP dispatch 有 provider 时调用 generate_byop_output
- [ ] 测试 BYOP dispatch 无 provider 时返回错误 stream
- [ ] 测试 custom_endpoints_as_providers bridge 正确转换
- [ ] 测试 lookup_byop 找到配置的 provider
- [ ] 验证: cargo test -p warp --lib -- byop (all pass)

---

## 并行/串行关系

```
独立可并行:
├── T11 (http_client features)
├── T13 (binary strip)
├── T14 (unit tests)
└── Windows rebuild

串行依赖链:
T9 (IDs) → T10 (auth types) → T12 (import gating) → 最终 no-cloud build

T15 依赖 T14
```

---

## 验证门控清单

| 门控 | 命令 | 预期 |
|------|------|------|
| Default build | `cargo check --bin warp-oss` | 0 errors |
| Local-only build | `cargo check --bin warp-oss --features local-only` | 0 errors |
| No-cloud build (目标) | `cargo check --bin warp-oss --no-default-features --features local-only` | 0 errors (T12完成后) |
| Unit tests | `cargo test -p warp_core -p warp_types -p genai` | all pass |
| Oss tests | `cargo test -p warp --lib -- oss` | all pass (T14完成后) |
| BYOP tests | `cargo test -p warp --lib -- byop` | all pass (T15完成后) |
| Dep tree | `cargo tree --no-default-features --features local-only \| grep -c cloud` | 0 (T12完成后) |
| Binary size | `ls -lh target/release/warp-oss` | < 400MB (T13完成后) |

---

## 量化预期

| 完成阶段 | Dep tree | Binary | 编译时间 |
|----------|----------|--------|---------|
| 当前 | 4378 lines | 711MB/326MB* | baseline |
| T13 (strip) | 不变 | **~384MB / ~190MB*** | 不变 |
| T9+T10 (types) | 不变 | 不变 | -5% (less recompile) |
| T12 (no-cloud) | **~2500 lines** | **~250MB / ~130MB*** | **-40%** |

*Linux debug / Windows release

---

## 补充审查: Crate #10-#14 结论

### #10: session-sharing-protocol (git dep, 纯数据类型)
- **状态**: 非 optional，5 crate 直接依赖 + app 50 文件
- **结论**: 零网络/IO，仅 serde+uuid+byte-unit 依赖。**编译成本极低**
- **发现**: warp_server_client 声明了依赖但**零实际 import** — 可立即删除
- **推荐**: 不优先处理；长期添加 `session_sharing` feature gate
- **可提取类型**: `SessionId`, `WindowSize`, `Role` 仅 4 处引用

### #11: warp-workflows (git dep, 333 generated workflow files)
- **状态**: 非 optional，被 app + cloud_object_models 使用
- **结论**: 269 个社区命令片段静态嵌入(5.5MB rlib)，零网络
- **关键发现**: `warp-workflows-types` 子 crate(Shell/Argument/Workflow 类型)必须保留
- **推荐**: 添加 `bundled_workflows` feature，将 5.5MB 静态数据变为可选
- **节省**: 3-5MB binary size + 330 编译单元

### #12: warp_isolation_platform (云隔离平台检测)
- **状态**: 非 optional，被 app(8处) + warp_managed_secrets(1处) 使用
- **结论**: detect() + issue_workload_token() 全部服务于云 agent sandbox
- **关键发现**: 唯一需要 warp_core 的原因是 1 行 channel 检查！可替换为 env var
- **推荐**: gate behind cloud_agents feature + 用 env var 替换 warp_core dep
- **收益**: 移除后 warp_core 不再被此 crate 间接拉入 managed_secrets 链

### #13: command-corrections (git dep, 52 规则引擎)
- **状态**: 非 optional，被 app + warp_core + warp_terminal 使用
- **结论**: 100% 本地/无网络，已被运行时设置 gate
- **关键发现**: warp_core 和 warp_terminal 仅用了 ExitCode 和 Shell 两个类型！
- **推荐**: 将 ExitCode 和 Shell 移到 warp_core → 去除反向依赖
- **优先级**: 低（唯一新 dep 是 difflib 270 行；其余已共享）

### #14: warp-command-signatures (JS 补全签名)
- **状态**: 已 optional (behind completions_v2 feature)
- **结论**: **无需额外操作** — 已正确 feature gate

---

## 补充发现: app/ 级别剩余瘦身机会

### 未 optional 的云 deps
| Dep | 可操作性 |
|-----|---------|
| cynic | server/ 8 子模块无 cfg gate → 随 T12 解决 |
| oauth2 | auth_manager.rs 无 cfg gate → 随 T12 解决 |
| qrcode | drive/ sharing 无 cfg gate → 随 drive gate 解决 |

### default features 中不应存在的(local-only 场景)
14 个语义上属于云的 feature flags 目前在 default 中:
cloud_environments, cloud_conversations, ambient_agents_rtc, cloud_mode,
oz_platform_skills, oz_identity_federation, sync_ambient_plans,
conversation_artifacts, oz_launch_modal, scheduled_ambient_agents,
oz_handoff, handoff_local_cloud, handoff_cloud_cloud, remote_codebase_indexing

### 58MB 嵌入资源可按 feature 排除
crates/warp_assets async/ 目录(41MB onboarding PNG)
已有 standalone exclude pattern → 复用到 local-only 即可

---

## BYOP 路径最终确认 ✅

所有关键 auth gate 均已正确 bypass:
- is_any_ai_enabled() → Channel::Oss bypass ✅
- is_byo_api_key_enabled() → early return true ✅
- is_custom_inference_enabled() → early return true ✅
- 8 处 is_logged_in() in ai/ → 全部有 && channel != Oss ✅
- Settings AI page → 3 处 bypass ✅
- **零未 bypass 的阻塞性 gate 剩余**

---

## 更新后的完整优先级排序

```
立即可做 (独立/并行):
├── T13: strip debuginfo (15 min)
├── T16: 从 warp_server_client 删除 session-sharing-protocol 依赖 (5 min)
├── T17: warp-workflows → optional bundled_workflows feature (1h)
├── T18: warp_isolation_platform → 替换 warp_core 为 env var (30 min)
├── T19: 14 个云 feature 移出 default (1h)
├── T20: warp_assets async/ exclude for local-only (15 min)
└── Windows rebuild (后台)

短期 (1-2 天):
├── T9: ID types → warp_types
├── T10: Auth types → warp_types
├── T11: http_client feature split
└── T14: Channel::Oss unit tests (20+ tests)

中期 (3-5 天):
├── T12: 196 文件 warp_graphql import gating (最大工作)
└── T15: BYOP 集成测试
```

---

## 补充审查: Crate #15-#16 结论

### #15: websocket (跨平台 WebSocket 客户端)
- **状态**: 非 optional，4 个 consumer
- **结论**: 唯一的 local-first 用途是 `remote_tty`(连接 ws://127.0.0.1:3030)
- **关键发现**: rustls+hyper+graphql-ws-client 全部仅服务于云路径
- **可 feature-gate 的组件**:
  - `proxy` feature → gate hyper + http-body-util (HTTP CONNECT tunnel)
  - `graphql` feature → gate graphql-ws-client
  - `tls` feature → gate rustls + tokio-rustls (remote_tty 用 ws:// 不需要 TLS)
  - `cloud-auth` feature → gate connect_with_headers + IAP helpers
- **推荐**: 添加 4 个 feature flag，minimal local build 仅需 async-tungstenite(plain)
- **潜在节省**: 去除 rustls/hyper/graphql-ws-client 约减少 30+ 传递 crate

### #16: prevent_sleep (防止系统休眠)
- **状态**: 被 http_client + server_api 使用(仅 2 处)
- **结论**: Linux 上是 zero-cost noop；macOS/Windows 有平台实现
- **关键发现**: 所有平台 deps 已 cfg-gate，零新增传递依赖
- **推荐**: **保持现状** — 编译成本极低，功能对长时间 AI 请求有用
- **优先级**: 无需操作

---

## 已完成任务更新

### 已完成 (commit 06bf0347):
- [x] T13: strip = "debuginfo" 添加到 [profile.release]
- [x] T16: 删除 warp_server_client 中未使用的 session-sharing-protocol 依赖
- [x] T17: warp-workflows 变为 optional (bundled_workflows feature)
- [x] T18: warp_isolation_platform 中 warp_core 变为 optional (env var 替代)
- [x] T19: 14 个云 feature 整合为 cloud_ui feature group
- [x] T20: warp_assets 添加 slim feature (排除 41MB async/ PNGs)

### websocket 瘦身计划 (T21, 中期):
- [ ] 添加 tls feature (default on) gate rustls/tokio-rustls
- [ ] 添加 proxy feature (default on) gate hyper/http-body-util  
- [ ] 添加 graphql feature (default on) gate graphql-ws-client
- [ ] 添加 cloud-auth feature (default on) gate connect_with_headers
- [ ] local-only build 可排除以上全部，仅保留 plain ws://

---

## 任务状态更新 (commit 362b3d90)

### 全部已完成:
- [x] T9: ID types (ClientId/SyncId/ServerId/HashableId/ToServerId) → warp_types
- [x] T10: Auth types (UserUid/UserMetadata/AnonymousUserType/PrincipalType) → warp_types
- [x] T11: http_client warp-cloud feature split (IAP/headers optional)
- [x] T13: strip = "debuginfo" 添加到 release profile
- [x] T14: 14 Channel::Oss 单元测试全部通过
- [x] T16: 删除 warp_server_client 幽灵 session-sharing-protocol 依赖
- [x] T17: warp-workflows optional (bundled_workflows feature)
- [x] T18: warp_isolation_platform 中 warp_core optional (env var fallback)
- [x] T19: 14 云 features 整合为 cloud_ui feature group
- [x] T20: warp_assets slim feature (exclude 41MB async/)
- [x] T21: websocket tls/proxy/graphql feature split

### 待执行:
- [ ] T12: 196 文件 warp_graphql import gating (最大工作, 3-5天)
  - Phase A: server/ 模块整体 cfg gate
  - Phase B: cloud_object/ 模块整体 cfg gate
  - Phase C: workspaces/ 模块整体 cfg gate
  - Phase D: drive/ 模块相关 imports
  - Phase E: 散落的 73 个 shared type imports
- [ ] T15: BYOP 集成测试

### warp_types crate 当前包含:
- ServerTimestamp, Uint32 (scalars)
- ClientId, ServerId, SyncId, ObjectUid, HashedSqliteId (IDs)
- HashableId, ToServerId traits + server_id_traits! macro
- UserUid (with lasso interner)
- UserMetadata, AnonymousUserType, PrincipalType, PersonalObjectLimits

---

## T12E 分析结论

### 为什么 no-default-features 有 1300 errors?

**根本原因**: `mod server` / `mod cloud_object` / `mod workspaces` 被 `#[cfg(feature = "cloud")]` 
gate 后，其他 408 个文件仍然无条件 `use crate::server::*` 等，导致级联失败。

### 最小可编译 feature set (0 errors):
```
local-only, cloud, bundled_workflows, full_source_code_embedding
```

### cloud feature 无法移除的原因:
- lib.rs 有 ~20 个无 cfg 的 `use crate::server::*` / `use crate::cloud_object::*`
- 6 个 external crates (warp_graphql 等) 被代码无条件引用
- 408 个文件 × 多个 import = 数千处需要 cfg 或 stub

### 修复策略 (分批):
1. Gate lib.rs 中 ~20 个 cloud imports (~1000 errors 减少)
2. 创建 stub modules (server/cloud_object/workspaces 的 no-op 版本)
3. 为 external crates 添加 feature-gated re-exports
4. Gate crates/ai 和 crates/persistence 中的 cloud imports

### 务实评估:
- 当前 `cargo check --bin warp-oss` (默认 build): ✅ 0 errors
- 当前 `--features local-only` (含 cloud): ✅ 0 errors, 6 warnings  
- 架构上已标记所有云模块为 optional
- BYOP AI 完全可用
- 真正的 no-cloud binary 需要 3-5 天的 stub 工作

### 下一步优先级调整:
1. ⭐ Windows rebuild + 用户测试验证 BYOP 能用
2. T15: BYOP 集成测试
3. T12E: 创建 stub modules (长期)
