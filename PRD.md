# PRD: Marb 云代码物理删除

## Problem Statement

Marb 是 Warp Terminal 的硬分叉，目标是一个纯 BYOP（自带 Provider）的本地终端。当前代码库中
仍残留大量云服务代码（~20 个 crate、~800 文件引用），虽已通过 cfg-gate 和 runtime check 隔离，
但带来三个实际痛点：

1. **编译慢** — dep tree 4378 行，大量从未执行的云 crate 参与编译
2. **认知噪音** — 打开 app/src/ 看到 server/、drive/、workspaces/ 等纯云目录，干扰开发者理解 BYOP 路径
3. **无验证手段** — 没有自动化冒烟测试，每次改动后只能手动启动确认"app 还能跑"，不可持续

## Solution

物理删除所有云代码（不是 cfg-gate），并建立自动化冒烟测试作为验证基线。执行完成后，Marb
代码库只包含 BYOP 路径实际需要的代码，编译更快、binary 更小、架构更清晰。

## User Stories

1. As a Marb developer, I want a `--smoke-test` CLI flag, so that I can自动验证 BYOP 对话链路在每次代码变更后仍然正常工作
2. As a Marb developer, I want dead cloud crates physically removed from workspace, so that `cargo check` 不再编译永远不会执行的代码
3. As a Marb developer, I want `app/src/drive/` deleted, so that Cloud Drive 分享功能的死代码不再占据目录空间和认知带宽
4. As a Marb developer, I want `app/src/workspaces/` deleted, so that团队 workspace 管理的代码不再与 BYOP 路径共存
5. As a Marb developer, I want server/graphql、server/iap、server/sync_queue、server/cloud_objects 删除, so that server/ 模块只剩 BYOP 路径实际依赖的基础设施
6. As a Marb developer, I want `server/ids` 的 import 路径改写为 `warp_types::`, so that 190 个文件不再依赖即将删除的 server 模块
7. As a Marb developer, I want telemetry 替换为本地 no-op stub, so that 154 个文件的 TelemetryEvent 引用不阻塞 server/ 删除
8. As a Marb developer, I want ServerApiProvider 中 BYOP 需要的接口提取到 `app/src/infra/`, so that AI 路径不再依赖 server_api 的云逻辑
9. As a Marb developer, I want每次删除是独立 commit 且可 revert, so that任何一步如果冒烟失败可以立即回滚到上一个已知可用状态
10. As a Marb developer, I want dep tree 行数随每个 phase 递减, so that我能量化每步删除的实际效果
11. As a Marb developer, I want release binary 体积在 Phase A 后显著减小, so that分发包更轻量
12. As a Marb developer, I want冒烟测试在 30 秒内完成（含 app 启动 + 一次 BYOP 往返）, so that验证循环足够快可以在每次 commit 后跑
13. As a Marb developer, I want冒烟测试使用 headless 模式（无 GPU 窗口）, so that未来可以在 CI 环境中运行
14. As a Marb developer, I want `server/` 目录最终完全删除, so that "server" 这个概念从 BYOP-only 的代码库中消失

## Implementation Decisions

### 策略：物理删除而非 cfg-gate

Marb 不 track upstream Warp。cfg-gate 是为了保持合并能力而存在的中间状态。既然不合并，
直接删除代码。这消除了 feature matrix 的复杂度、CI 的多 feature 组合测试负担、以及
每个新贡献者理解"哪些 feature 打开哪些不打开"的认知成本。

### 冒烟测试架构

- 新增 `LaunchMode::SmokeTest` 变体，复用 `AppBuilder::new_headless()` 路径
- 完整 singleton 初始化（包括 ServerApiProvider、AI 子系统），不启动 GPU 窗口
- 初始化完成后，程序化注入一条 user message ("ping") 进入 BYOP 路径
- 等待 stream 完成事件，检查日志 `[byop] stream stats: start=1 chunks=N` (N>0)
- 成功 exit 0，失败 exit 1（含 30 秒超时）
- 使用真实 BYOP endpoint（ds-api.xnurta.com），不用 mock

### 删除分层

三层结构，按风险递增排列：

**Layer 0（冒烟测试）**：先建立验证基线，再动手删除

**Layer 1（workspace crate 删除）**：删除 warp_server_client、warp_server_auth、firebase_auth、
cloud_object_persistence、warp_graphql、cloud_object_client、managed_secrets_wasm / warp_managed_secrets。
不动 app/src/ 代码逻辑，只移除 Cargo.toml 引用和 crate 目录。

**Layer 2（app module 删除）**：先删纯云模块（drive/、workspaces/、server 子目录），
再迁移 BYOP 依赖的接口（server_api → infra/，telemetry → no-op stub，ids → warp_types），
最后删除 server/ 空壳。

### 粒度和回滚

每个模块/crate 的删除是一个独立 git commit。每个 commit 后运行 cargo check + 冒烟测试。
失败时 `git revert HEAD` 回到上一个可用状态，诊断问题后重试。

### server_api 迁移策略

`ServerApiProvider` 是一个 singleton struct，提供 `get_ai_client()` 等 trait object accessor。
BYOP 路径实际只需要 `AIClient` trait。迁移策略：
- 在 `app/src/infra/` 创建 `ServiceProvider` struct（精简版，只保留 BYOP 需要的 accessor）
- `AIClient` trait 定义移入 infra/
- 其余 cloud-only client（WorkspaceClient、TeamClient、ReferralsClient 等）随 server/ 一起删除

### telemetry 迁移策略

`TelemetryEvent` 是一个大 enum（1268+ 行定义），被 154 个文件引用。迁移策略：
- 创建 `app/src/telemetry.rs`，包含同名 enum，所有 variant 保留但 `send()` 方法是 no-op
- 机械替换 import 路径
- 未来 Phase B2 可以决定是否需要本地 telemetry（写文件/metrics）

## Testing Decisions

### 什么是好的测试

本项目的测试只验证外部可观察行为：**BYOP 对话能否端到端完成**。不测试内部实现细节
（provider 解析的中间步骤、singleton 注册顺序等）。

### 测试接缝

1. **主接缝：`warp-oss --smoke-test`** — 最高层级，验证完整初始化链 + BYOP 流。如果这里过了，所有下层都在工作。
2. **快速失败接缝：`cargo check -p warp`** — 编译级守门员，10 倍速度于冒烟测试，作为第一道筛选。

### 不加额外单元级接缝的原因

- 当前零 Oss 测试基础设施，从头搭 mock 成本高
- 冒烟测试已隐式覆盖 provider resolution — 如果解析坏了 stream stats 不会出现
- 细粒度测试留给 Phase B2（架构清理后接口更稳定）

### 先例

项目中已有 `LaunchMode::Test` + `TestDriver` 机制用于集成测试（见 `run_integration_test()`），
冒烟测试复用相同的 headless 初始化路径。

## Out of Scope

- **Provider model 统一**（CustomEndpoint vs AgentProvider 合并）— 留给 Phase B2
- **chat_stream.rs 拆分**（7314 行 god file）— 留给 Phase B2
- **BlocklistAIController 拆分**（17 个 send 方法）— 留给 Phase B2
- **BYOP 单元/集成测试**（mock provider、error path 覆盖）— 留给 Phase B2
- **CI/CD 集成**（GitHub Actions 自动跑冒烟）— 留给冒烟测试稳定后
- **跨平台验证**（macOS/Linux build）— 当前只在 Windows 验证
- **UI 自动化测试**（Settings 页面渲染、API type 显示）— 不在当前 scope

## Further Notes

### 量化预期

| 完成阶段 | Dep tree 行数 | Binary 体积 | 编译时间 |
|----------|--------------|-------------|---------|
| 当前 | ~4378 | ~326MB (Win release) | baseline |
| Phase A 后 | ~2500-3000 | ~220-250MB | -20~30% |
| Phase B 后 | ~2000 | ~180-200MB | -40% |

### 风险和缓解

| 风险 | 缓解 |
|------|------|
| 删除 crate 后 app 路径隐式依赖它 | 冒烟测试 catch；单 commit 可 revert |
| server_api 迁移遗漏某个 BYOP 调用点 | grep 扫描所有 `ServerApiProvider` 引用，逐个确认 |
| telemetry no-op stub 的 enum variant 不全 | 编译错误会立即暴露缺失 variant |
| headless 模式下 BYOP init 路径和 GUI 模式不同 | `LaunchMode::SmokeTest` 复用 `LaunchMode::App` 的 singleton 注册逻辑 |
| 真实 endpoint 偶发超时导致冒烟假阴性 | 30 秒超时 + 允许 1 次重试 |

### 执行顺序快速参考

```
Phase C: --smoke-test 基础设施
Phase A: A1(warp_server_client) → A2(warp_server_auth) → A3(firebase_auth)
         → A4(cloud_object_persistence) → A5(warp_graphql) → A6(cloud_object_client)
         → A7(managed_secrets)
Phase B: B1(drive/) → B2(workspaces/) → B3(server 子目录)
         → B4(ids rewrite) → B5(telemetry stub) → B6(server_api extraction)
         → B7(delete server/)
```
