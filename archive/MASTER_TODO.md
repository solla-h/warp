# Master TODO — 更新版 (T5/T6完成后)

## 完成状态

### 已完成 ✅
- [x] P0: BYOP Direct Provider (genai + agent_providers + response_stream seam)
- [x] P0-UX: Settings UI bridge (custom_endpoints → BYOP)
- [x] P1.1-P1.5: 全部云模块 runtime gate
- [x] P2.1-P2.4: firebase/cloud_object_persistence/warp_server_auth/warp_server_client optional
- [x] P3: Warnings 304→6 (local-only), 137→0 (default)
- [x] P4: 15张云表 cfg gate
- [x] T1: OpenTelemetry 5 deps → optional (otel feature)
- [x] T2: warp_managed_secrets → optional (cloud feature)
- [x] T3: AWS SDK 4 deps → optional (aws-bedrock feature)
- [x] T4: Workspace exclude (managed_secrets_wasm, integration)
- [x] T5: ServerTimestamp/Uint32 → warp_types crate
- [x] T6: warp_graphql → optional in app
- [x] T7: Gate onboarding/RemoteServer/cloud CLI commands
- [x] Critical fix: runtime Channel::Oss check (替换 cfg!)
- [x] Critical fix: 8处 AI login gate bypass
- [x] Critical fix: Settings AI page auth gate bypass
- [x] Critical fix: is_any_ai_enabled() bypass

### 进行中 🔄
- [ ] T8: cloud_objects → optional in app (102文件引用)
- [ ] Windows rebuild (dc233224 commit)

### 待做 📋
- [ ] T0 验证: 实际运行 /agent (等Windows build)
- [ ] T8完成后: 验证 dep tree 真正缩小
- [ ] 最终打包: exe + DLLs

---

## T8: cloud_objects Optional — 最终目标

### 挑战
- 102个文件引用 `cloud_objects`
- 它是当前唯一阻止 warp_graphql 从 dep tree 消失的 transitive 链
- 深层类型依赖: `SyncId`, `ClientId`, `ServerId`, `ObjectUid` 等 ID 类型

### 策略
1. 先 inline 或 re-export cloud_objects 的共享 ID 类型到 warp_types
2. 然后让 cloud_objects 变 optional
3. cfg gate 所有直接引用

### 预期效果 (T8完成后)
- dep tree 减少 ~50+ crate
- binary ~220MB (from 326MB)
- 编译加速 ~30%
