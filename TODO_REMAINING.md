# 待办事项清单 (TODO)

## 紧急 — 功能阻断类

### ✅ 已修复
- [x] Onboarding 跳过 (runtime Channel::Oss check) — `fec3f9cb`
- [x] Login bypass for agent driver/mod — `fec3f9cb`
- [x] 8处 AI login gates bypass — `e339378e`
- [x] FreeAvailableModels no-op in Oss mode — `34a24eb3`

### ⚠️ 部署/打包
- [ ] ConPTY DLL: `conpty.dll` 必须放在 exe 旁边 (app/assets/windows/x64/)
- [ ] 其他 DLL: vcruntime140.dll, vcruntime140_1.dll, msvcp140.dll, dxcompiler.dll, dxil.dll
- [ ] 打包脚本: 需要自动化 copy DLLs 到 output 目录

### ⚠️ 可能阻断的残留 login gate
- [ ] `ai/harness_availability.rs` — harness auth secret fetch (已加 bypass 但需运行时验证)
- [ ] `ai/agent_sdk/admin.rs` — SDK credentials (已加 bypass 但需验证)
- [ ] root_view.rs 中还有 3 处 is_logged_in 检查 (非AI路径，影响 UI 显示)
- [ ] settings/privacy.rs ×4 — 隐私设置同步 (cosmetic, 不阻断)
- [ ] workspaces/update_manager.rs ×2 — workspace 更新 (已 gate init)

---

## 瘦身优化 — 编译时间/体积

### P2: 让云 crate 真正 optional (减 dep tree)

#### 已完成
- [x] P2.1-P2.2: cloud_object_persistence optional + firebase import gated
- [x] P2.3: warp_server_auth optional
- [x] P2.4: warp_server_client optional

#### 未完成 (需修改中间 crate)
- [ ] P2.5: `warp_graphql` optional (1378 subtree deps — cynic, prost)
- [ ] P2.6: `warp_managed_secrets` optional (1583 subtree deps — tink, hpke)
- [ ] P2.7: `aws-config` + `aws-sdk-sts` optional (589 deps)
- [ ] P2.8: `opentelemetry-*` optional (108 deps)
- [ ] P2.9: `oauth2` optional in http_client (145 deps)

### P4: Schema 精简
- [x] 15张云表已 cfg gate behind persistence/cloud feature

### P5: Workspace 排除
- [ ] Exclude 9 unused crates: voice_input, firebase, warp_server_client,
      command-signatures-v2, warp_js, warp_web_event_bus, managed_secrets_wasm,
      virtual_fs, integration

### 运行时 gate (已完成)
- [x] billing/pricing stub
- [x] drive init gate
- [x] TelemetryCollector gate
- [x] cloud_object init gate (5 singletons)
- [x] workspaces init gate (7 registrations)
- [x] server init gate (ServerApiProvider + 10 deps)

### 可进一步 gate 的模块
- [ ] `agent_sdk/` 10个纯云文件 (OAuth, API key, schedule, environment)
- [ ] `remote_server/` 24 files — 整体 mod gate
- [ ] Settings 6个云页面隐藏 (billing, environments, platform, referrals, teams, warp_drive)
- [ ] `onboarding::init(ctx)` 调用 gate (仅 UI cosmetic)

---

## 功能完善

### BYOP
- [ ] Settings UI 完整 provider 配置界面 (当前需手动编辑 settings.toml 或用 Custom Endpoints)
- [ ] 验证 Ollama/DeepSeek/本地模型连接
- [ ] byop_compaction 运行时验证 (context overflow 处理)
- [ ] 工具调用验证 (read_files, run_shell_command, grep, apply_file_diffs)

### 自定义编排
- [ ] Orchestrator trait 设计
- [ ] 自定义 tool 注册机制
- [ ] Prompt 模板自定义 (.j2 文件可编辑)

### 用户体验
- [ ] 首次启动引导: 快速配置 provider 的 minimal wizard
- [ ] 错误提示: 未配置 provider 时给出明确指引
- [ ] 模型选择器: 显示 BYOP 模型名称

---

## Transitive Dependency 瓶颈

```
warp (app)
└── cloud_objects (NOT optional, hard dep)
    └── warp_server_auth (transitive)
        └── warp_managed_secrets (tink, hpke)
└── cloud_object_client (NOT optional)
    └── cloud_object_models
        └── cloud_object_persistence
└── warp_graphql (NOT optional)
    └── cynic, prost, graphql-ws-client
```

完全消除这些 transitive deps 需要:
1. B1: 从 cloud_objects 移除 warp_server_auth (已验证可行 — inline UserUid)
2. B2-B3: 从 warp_server_client 移除 phantom deps (已验证)
3. 让 cloud_objects 本身 optional in app (最终目标 — 需 102 文件改动)
