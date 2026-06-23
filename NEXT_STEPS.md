# 后续执行计划 (P2-P5)

## P2: 云 Crate 变 Optional Dependency

### 目标
让 `cargo build --bin warp-oss --no-default-features --features local-only` 真正不编译云 crate，减少 ~30% 编译时间和二进制体积。

### 前置条件
所有 `use cloud_crate::` 语句必须被 `#[cfg(not(feature = "local-only"))]` 包裹。

### 执行步骤

#### P2.1: Gate `firebase` imports (最简单，1 个 use-import)
- [ ] 找到 `use firebase::` 的位置（server/sync_queue_tests.rs）
- [ ] 添加 `#[cfg(not(feature = "local-only"))]`
- [ ] 在 app/Cargo.toml 中改为 `firebase = { workspace = true, optional = true }`
- [ ] 添加到 features: `cloud = ["dep:firebase", ...]`
- [ ] 验证: `cargo check --bin warp-oss` (0 errors)
- [ ] 验证: `cargo check --bin warp-oss --features local-only` (0 errors)
- [ ] 验证: `cargo tree --bin warp-oss --features local-only | grep firebase` (应无输出)

#### P2.2: Gate `cloud_object_persistence` imports (2 文件)
- [ ] Gate `use cloud_object_persistence::` in persistence/sqlite.rs
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.3: Gate `warp_server_auth` imports (4 文件)
- [ ] Gate crash_reporting/, auth/, drive/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.4: Gate `warp_server_client` imports (8 文件)
- [ ] Gate server/, drive/, workspace/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.5: Gate `cloud_object_client` imports (16 文件)
- [ ] Gate cloud_object/, server/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.6: Gate `cloud_object_models` imports (25 文件)
- [ ] Gate settings/, workflows/, env_vars/, workspaces/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.7: Gate `warp_managed_secrets` imports (29 文件)
- [ ] Gate lib.rs, server/, tracing/, ai/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.8: Gate `warp_graphql` imports (78 文件 — 最大)
- [ ] Gate root_view.rs, persistence/, settings/, server/ 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

#### P2.9: Gate `cloud_objects` imports (102 文件 — 最难)
- [ ] Gate cloud_object/, drive/, ai/, server/ids.rs 中的引用
- [ ] 改为 optional dep
- [ ] 验证三项门控

### 最终验证
- [ ] `cargo tree --bin warp-oss --no-default-features --features local-only | grep -c "warp_graphql\|firebase\|cloud_object"` → 0
- [ ] 编译时间对比 (default vs local-only)
- [ ] 二进制体积对比

---

## P3: 清理 Warnings (304 个 dead-code warnings)

### 目标
消除 `--features local-only` 模式下的 304 个 warnings。

### 执行步骤

#### P3.1: 分类 warnings
- [ ] `cargo check --bin warp-oss --features local-only 2>&1 | grep "^warning" | sed 's/warning: //' | sort | uniq -c | sort -rn | head -20`
- [ ] 识别 top warning 类别 (unused imports, dead code, unused variables)

#### P3.2: 批量修复 unused imports
- [ ] 对每个 `unused import` warning，在对应的 use 语句加 `#[cfg(not(feature = "local-only"))]`
- [ ] 或使用 `#[allow(unused_imports)]` 在文件级别（如果太多）
- [ ] 验证: warning 数量从 304 降到 < 50

#### P3.3: 批量修复 dead code
- [ ] 对 `function is never used` warnings，加 `#[cfg(not(feature = "local-only"))]` 或 `#[allow(dead_code)]`
- [ ] 验证: warning 数量 < 20

#### P3.4: 最终清理
- [ ] 手动检查剩余 warnings，逐个修复
- [ ] 验证: `cargo check --features local-only 2>&1 | grep "warning:" | wc -l` < 10

### 门控
- [ ] Cloud build warnings 不增加 (保持 137)
- [ ] Local-only build warnings < 10
- [ ] 无新 errors

---

## P4: Persistence Schema 精简

### 目标
Gate 16 张纯云表，使 local-only 构建不生成/迁移这些表。

### 前置条件
P2 完成（cloud_object_persistence 已 optional）

### 执行步骤

#### P4.1: 创建条件 schema
- [ ] 在 crates/persistence/src/schema.rs 中对 16 张云表添加 `#[cfg(not(feature = "local-only"))]`
- [ ] 对应的 model.rs 中的 struct 也加 cfg
- [ ] 验证编译通过

#### P4.2: 条件 migrations
- [ ] 在 crates/persistence/src/lib.rs 中条件加载 migrations
- [ ] local-only 只运行 29 张必需表的 migrations
- [ ] 验证: 新建 SQLite DB 时只有 29 张表

#### P4.3: 验证数据完整性
- [ ] 在 local-only 模式创建新 DB
- [ ] 验证所有必需表存在 (agent_conversations, blocks, windows, etc.)
- [ ] 验证云表不存在 (teams, cloud_objects_refreshes, etc.)

### 云表列表 (16 张)
cloud_objects_refreshes, current_user_information, folders, notebooks,
object_actions, object_metadata, object_permissions, project_rules,
server_experiments, team_members, team_settings, teams, user_profiles,
users, workspace_teams, workspaces

### 门控
- [ ] Default build: 50 张表全在
- [ ] Local-only build: 29+5=34 张表 (必需 + uncertain)
- [ ] 已有 terminal/AI 功能不受影响

---

## P5: 删除纯云 Crate 源码

### 目标
从 workspace 中排除不再需要的 crate 目录，减少仓库体积和 IDE 索引负担。

### 前置条件
P2 完成（这些 crate 已是 optional 且 local-only 不链接）

### 执行步骤

#### P5.1: 从 workspace members 中移除
- [ ] 编辑 Cargo.toml `[workspace] members` 列表
- [ ] 将以下 crate 移到 `exclude` 列表:
  - `crates/graphql` (warp_graphql)
  - `crates/warp_graphql_schema`
  - `crates/firebase`
  - `crates/cloud_object_client`
  - `crates/cloud_object_models`
  - `crates/cloud_object_persistence`
  - `crates/cloud_objects`
  - `crates/warp_server_auth`
  - `crates/warp_server_client`
  - `crates/managed_secrets` (warp_managed_secrets)
  - `crates/managed_secrets_wasm`
  - `crates/field_mask`
  - `crates/channel_versions`
- [ ] 验证: `cargo check --bin warp-oss` (0 errors)
- [ ] 验证: `cargo check --bin warp-oss --features local-only` (0 errors)

#### P5.2: 可选 — 物理删除目录
- [ ] 备份到 archive 分支
- [ ] `rm -rf crates/graphql crates/firebase ...`
- [ ] 验证编译通过

#### P5.3: 最终体积对比
- [ ] `du -sh .` before vs after
- [ ] `cargo tree --bin warp-oss | wc -l` before vs after
- [ ] 编译时间对比

### 门控
- [ ] Default build (with cloud feature): 仍可编译 (从 git 拉取或保留为 git dep)
- [ ] Local-only build: 编译不依赖这些 crate
- [ ] `cargo tree --bin warp-oss --features local-only` 不包含云 crate

---

## 执行优先级和依赖关系

```
P2 (optional deps) ──→ P4 (schema) ──→ P5 (delete crates)
                  └──→ P3 (warnings) [可并行]
```

P3 可以和 P2 并行做（不互相依赖）。
P4 和 P5 必须在 P2 完成后才能做。
