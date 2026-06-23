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
