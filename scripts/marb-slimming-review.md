# Marb 瘦身审查报告

> 审查日期: 2026-06-30
> 审查分支: `marb` (commit `f5e8968b`)
> 审查基线: 62 commit 云代码删除完成后，工作树干净

---

## 审查总结

| 维度 | 数据 |
|------|------|
| **审查范围** | 整个代码库（非增量 diff 审查） |
| **已完成的瘦身工作** | T9-T21 全部完成（ID 类型提取、http_client feature split、websocket feature split、14 云 feature 整合为 `cloud_ui`、binary strip 等） |
| **未完成的已有计划** | T12（196 文件 import gating）、T15（BYOP 集成测试） |
| **本次新发现** | 16 个问题（P0: 1, P1: 5, P2: 7, P3: 3）+ 5 个需要代码改动的中期删除项 |
| **立即可回收空间** | 工作树 ~500 MB，git 追踪文件 ~175 MB |

**整体评估**: COMMENT — 存在大量可立即清理的死代码、死依赖、二进制文件和陈旧文档，且均为零编译风险操作。

---

## 严重级别定义

| 级别 | 名称 | 说明 |
|------|------|------|
| **P0** | 关键 | 必须立即删除（大体积二进制、明确的垃圾文件） |
| **P1** | 高 | 当前迭代删除（死依赖、陈旧文档、不应入库的缓存） |
| **P2** | 中 | 后续迭代（需要 feature gate 或确认后删除） |
| **P3** | 低 | 可选改进（清理噪音、品牌替换） |

---

## P0 — 关键（立即删除）

### 1. PDB 调试符号文件被 git 追踪（73.5 MB）

| 字段 | 详情 |
|------|------|
| **位置** | `app/assets/windows/{x64,arm64}/{OpenConsole,conpty}.pdb` |
| **体积** | x64/OpenConsole.pdb = 32.4 MB, arm64/OpenConsole.pdb = 28.8 MB, x64/conpty.pdb = 6.2 MB, arm64/conpty.pdb = 6.1 MB, 合计 **73.5 MB** |
| **理由** | PDB 是 Windows 调试符号文件，运行时完全不需要，仅在调试 ConPTY/OpenConsole 源码时有用，应由开发者本地生成或单独下载 |
| **证据** | `git ls-files "*.pdb"` 确认 4 个文件被追踪 |
| **删除步骤** | `git rm app/assets/windows/*/OpenConsole.pdb app/assets/windows/*/conpty.pdb`，在 `.gitignore` 添加 `*.pdb` |
| **验证** | `cargo check --bin warp-oss` 仍通过（PDB 不参与编译） |

---

## P1 — 高（当前迭代删除）

### 2. `specs/` 目录 — 410 个 Warp 内部规格文档（5.5 MB）

| 字段 | 详情 |
|------|------|
| **位置** | `specs/` (整个目录) |
| **体积** | 221 个子目录，410 个被 git 追踪的文件，5.5 MB |
| **理由** | 全是 Warp 原团队 Jira/GitHub issue 规格文档（如 `APP-1915/`, `REMOTE-1453/`, `QUALITY-701/`），与 Marb 的 BYOP 终端目标完全无关 |
| **证据** | `git ls-files specs \| Measure-Object` → 410 文件；交接文档"下一步建议"第 2 条已提及 |
| **删除步骤** | `git rm -r specs/` |
| **验证** | 无编译影响（纯文档目录） |

### 3. `tink-rust` 死 patch（3 个 `[patch.crates-io]` 条目）

| 字段 | 详情 |
|------|------|
| **位置** | `Cargo.toml:547-549` |
| **内容** | `tink-core`, `tink-proto`, `tink-hybrid` 三个 patch 条目 |
| **理由** | tink 是 Google 加密库，原用于 `warp_managed_secrets`（已删除）。当前全代码库零 `.rs` 文件引用 tink，零 `Cargo.toml` 将 tink 列为依赖（仅作为 patch 存在） |
| **证据** | `rg "tink" --type rust` → 0 结果；`rg "^tink" **/Cargo.toml` → 仅 patch 条目 |
| **删除步骤** | 删除 `Cargo.toml` 第 547-549 行的 3 行 tink patch |
| **验证** | `cargo check --bin warp-oss` 通过 |

### 4. 根目录陈旧文档和构建日志（~170 KB，11 个文件）

| 文件 | 大小 | 建议操作 |
|------|------|----------|
| `build_log.txt` | 57.4 KB | 删除（构建日志） |
| `build_log2.txt` | 21.5 KB | 删除（构建日志） |
| `DEPENDENCY_GRAPH.md` | 20.0 KB | 删除（旧依赖图快照，已过时） |
| `LOCAL_FIRST.md` | 15.4 KB | 删除或移入 `docs/`（早期设计文档，已被 PRD.md 取代） |
| `WARP.md` | 12.7 KB | 删除（Warp 原始文档） |
| `PRD.md` | 8.4 KB | 移入 `docs/` 或保留 |
| `PLAN.md` | 7.4 KB | 删除（旧计划，已被 archive/ 取代） |
| `NEXT_STEPS.md` | 6.7 KB | 删除（P2-P5 计划，大部分已完成） |
| `CODE_REVIEW_ISSUES.md` | 6.1 KB | 删除（旧审查问题） |
| `CONTEXT.md` | 5.0 KB | 删除或移入 `docs/` |
| `TODO_REMAINING.md` | 3.9 KB | 删除（待办清单，大部分已完成） |

| **删除步骤** | `git rm build_log.txt build_log2.txt DEPENDENCY_GRAPH.md PLAN.md NEXT_STEPS.md CODE_REVIEW_ISSUES.md TODO_REMAINING.md`；将 `PRD.md`、`LOCAL_FIRST.md`、`WARP.md`、`CONTEXT.md` 移入 `docs/`（或删除） |
| **验证** | 无编译影响 |

### 5. `archive/` 目录 — 过时计划文档

| 字段 | 详情 |
|------|------|
| **位置** | `archive/MASTER_TODO.md`, `archive/SLIM_PLAN_V2.md` |
| **理由** | 两份瘦身计划文档，内容已被本审查取代。MASTER_TODO 中 T8 仍待做但信息已过时；SLIM_PLAN_V2 的任务全部完成或转入 TODO_REMAINING |
| **删除步骤** | `git rm -r archive/`（或将有用信息提取到单一 `docs/SLIMMING.md` 后删除） |

### 6. `command-signatures-v2/js/.yarn/cache/` — 巨大 JS 缓存（git 历史中 ~80 MB）

| 字段 | 详情 |
|------|------|
| **位置** | `crates/command-signatures-v2/js/.yarn/cache/` |
| **体积** | git 历史中最大的两个 blob：`typescript-patch-39146.zip` (40.7 MB) + `typescript-npm-5.2.2.zip` (40.6 MB) = ~80 MB |
| **理由** | Yarn 包管理器缓存是可重建的派生产物，不应进入版本控制。`command-signatures-v2` crate 已是 optional（behind `completions_v2` feature），其 JS 构建产物更不应在仓库中 |
| **删除步骤** | `git rm -r crates/command-signatures-v2/js/.yarn/cache/`，添加到 `.gitignore` |
| **注意** | 要彻底从历史中清除需要 `git filter-repo`，但当前分支删除即可停止增长 |

---

## P2 — 中（后续迭代）

### 7. `cloud` feature 仍在 `default` 中 — 云代码仍参与编译

| 字段 | 详情 |
|------|------|
| **位置** | `app/Cargo.toml:507-686` (`default = ["otel", "cloud", "aws-bedrock", ...]`) |
| **现状** | T12（196 文件 import gating）未完成，`cloud` feature 无法从 default 移除 |
| **影响** | dep tree 仍含 ~50+ 云传递依赖；binary 仍约 326 MB（Windows release） |
| **建议** | 按 SLIM_PLAN_V2 的 T12 分 5 阶段执行（Phase A-E），这是已有计划中最大的剩余工作项 |

### 8. `remote_server` crate — 非 optional 的纯云依赖

| 字段 | 详情 |
|------|------|
| **位置** | `crates/remote_server/` (24 个源文件) |
| **依赖链** | 被 `app/Cargo.toml:266` 和 `crates/warp_files/Cargo.toml:16` 非 optional 引用 |
| **理由** | 纯云基础设施：远程服务器管理、SSH 传输、codebase 索引协议。Marb 作为本地 BYOP 终端不需要 |
| **执行步骤** | 1) `warp_files` 中 gate `remote_server` 引用 2) app 中改为 optional 3) 加入 `cloud` feature group |
| **工作量** | 1 天 |

### 9. `warp_graphql_schema` — 可删除的 GraphQL schema crate

| 字段 | 详情 |
|------|------|
| **位置** | `crates/warp_graphql_schema/` |
| **理由** | 被 `warp_types` 在 `graphql` feature 下引用（`graphql = ["dep:cynic", "dep:warp_graphql_schema"]`），但该 feature 未被任何 crate 启用 |
| **证据** | `rg "features.*graphql" **/Cargo.toml` → 0 结果 |
| **建议** | 确认 `warp_types` 的 `graphql` feature 确实无消费者后，从 workspace 删除此 crate |
| **工作量** | 0.5 天 |

### 10. `.agents/skills/` — Warp 团队内部 AI 技能定义

| 字段 | 详情 |
|------|------|
| **位置** | `.agents/skills/` (14 个 skill 目录) |
| **内容** | `add-feature-flag`, `add-telemetry`, `classify-changelog-pr`, `create-launch-modal`, `promote-feature` 等 |
| **理由** | Warp 内部工作流技能，引用 Warp 的 Jira/Linear 工具链，与 Marb 的开发流程无关 |
| **建议** | 删除或替换为 Marb 自己的技能定义 |

### 11. 集成测试数据中的云相关 SQLite（14.3 MB）

| 字段 | 详情 |
|------|------|
| **位置** | `crates/integration/tests/data/cloud_objects.sqlite` |
| **体积** | 14.3 MB |
| **理由** | 云对象测试数据库。`crates/integration` 已被 workspace exclude，测试数据不参与常规构建 |
| **删除步骤** | `git rm crates/integration/tests/data/cloud_objects.sqlite` |

### 12. 终端 ref_tests 大型 JSON 数据（~41 MB）

| 字段 | 详情 |
|------|------|
| **位置** | `app/src/terminal/ref_tests/data/` (160 文件) |
| **体积** | 包含 `grid.json` 文件（18 MB、12 MB、11 MB 等），合计 ~41 MB |
| **说明** | 终端渲染回归测试的快照数据，是有效测试数据 |
| **建议** | **不建议删除**。但可考虑 Git LFS 管理以减小 clone 体积 |

### 13. `docker/agent-dev/` — 云 Agent 开发环境

| 字段 | 详情 |
|------|------|
| **位置** | `docker/agent-dev/Dockerfile` |
| **理由** | 云 Agent 开发环境，Marb 不使用云 Agent |
| **建议** | 删除 `docker/agent-dev/`，保留 `docker/linux-dev/`（本地 Linux 构建用） |

---

## P3 — 低（可选改进）

### 14. `.git-rewrite/` 目录（775 KB）

| 字段 | 详情 |
|------|------|
| **位置** | `.git-rewrite/map/` |
| **理由** | `git filter-repo` 的残留暂存目录，操作完成后应删除 |
| **建议** | 确认 filter-repo 操作已完成后删除本地目录 |

### 15. `images/` — Warp 品牌图片

| 字段 | 详情 |
|------|------|
| **位置** | `images/Built-With-Warp-Export@2x.png`, `images/Powered-By-Oz-Export@2x.png` |
| **理由** | Warp/Oz 品牌图片，Marb 已完成品牌重命名（Zap→Marb） |
| **建议** | 删除或替换为 Marb 品牌 |

### 16. Warp 专属 GitHub 配置

| 字段 | 详情 |
|------|------|
| **位置** | `.github/` 下的 issue templates、PR templates、triage 配置等 |
| **理由** | 引用 Warp 的 Jira、Linear、内部工具链，对 Marb 无效 |
| **建议** | 清理为 Marb 自己的模板，或删除 |

---

## 删除执行计划

### 第一阶段：可立即安全删除（无编译影响，独立操作）

以下操作全部是纯文件删除，零编译风险，可并行执行：

| # | 项目 | 节省空间 | 命令 |
|---|------|----------|------|
| 1 | PDB 文件 (4个) | 73.5 MB | `git rm app/assets/windows/*/OpenConsole.pdb app/assets/windows/*/conpty.pdb` + `.gitignore` 加 `*.pdb` |
| 2 | `specs/` 目录 | 5.5 MB + 410 文件 | `git rm -r specs/` |
| 3 | tink patch (3行) | 清理噪音 | 编辑 `Cargo.toml` 删除 547-549 行 |
| 4 | `build_log*.txt` | 79 KB | `git rm build_log.txt build_log2.txt` |
| 5 | 陈旧根 .md 文件 | ~70 KB | `git rm DEPENDENCY_GRAPH.md PLAN.md NEXT_STEPS.md CODE_REVIEW_ISSUES.md TODO_REMAINING.md` |
| 6 | `archive/` 目录 | 2 文件 | `git rm -r archive/` |
| 7 | `.yarn/cache/` | ~80 MB git 历史 | `git rm -r crates/command-signatures-v2/js/.yarn/cache/` + `.gitignore` |
| 8 | `.agents/skills/` | 14 目录 | `git rm -r .agents/skills/` |
| 9 | `docker/agent-dev/` | 1 文件 | `git rm -r docker/agent-dev/` |
| 10 | `.git-rewrite/` | 775 KB | 删除本地目录 |
| 11 | `images/` | 2 图片 | `git rm -r images/` |
| 12 | `cloud_objects.sqlite` | 14.3 MB | `git rm crates/integration/tests/data/cloud_objects.sqlite` |
| 13 | `portable_warp_oss/` | 397 MB 本地 | 删除本地目录（已在 .gitignore，未被 git 追踪） |

**预计立即可回收**: 工作树 ~500 MB，git 追踪文件 ~175 MB

### 第二阶段：需要代码改动的删除（有编译影响）

| # | 项目 | 前置条件 | 工作量 |
|---|------|----------|--------|
| A | `remote_server` → optional | gate warp_files + app 引用 | 1 天 |
| B | `warp_graphql_schema` 删除 | 确认 graphql feature 无消费者 | 0.5 天 |
| C | `cloud` 移出 default (T12) | 196 文件 import gating | 3-5 天 |
| D | `cloud_objects` / `cloud_object_models` 物理删除 | T12 完成 | 0.5 天 |
| E | `cynic` 依赖移除 | T12 完成（cynic 仅通过云 crate 间接引入） | 随 T12 |

### 第三阶段：Git 历史瘦身（可选，长期）

1. **`git filter-repo` 清理历史**: 完成上述删除后，git 历史中仍有多个 40 MB+ 的 blob（yarn cache、PDB）。用 `git filter-repo` 清理可将 `.git` 目录从数 GB 缩减到几百 MB。

2. **Git LFS 迁移**: 以下大文件建议迁移到 Git LFS 管理：
   - `dxcompiler.dll`（22 MB + 18 MB，arm64/x64）
   - `rustyrain.gif`（18 MB，warpui 示例资源）
   - `bert_tiny_v1.onnx`（17 MB，input_classifier 模型）
   - 终端 ref_tests JSON（41 MB，160 文件）

### 验证门控

每完成一个阶段的删除后，执行以下验证：

| 门控 | 命令 | 预期 |
|------|------|------|
| 默认构建 | `cargo check --bin warp-oss` | 0 errors |
| Local-only 构建 | `cargo check --bin warp-oss --features local-only` | 0 errors |
| 单元测试 | `cargo test -p warp --lib -- oss` | all pass |
| Binary 体积 | 检查 `target/release/warp-oss.exe` | 持续下降 |
| Dep tree | `cargo tree --bin warp-oss \| wc -l` | 持续下降 |

---

## 额外建议

1. **`.cargo/config.toml` 优化**: 考虑添加 `[build] rustflags = ["-C", "debuginfo=0"]` 用于 local-only 构建进一步减小 binary。

2. **Workspace exclude 扩展**: 当前 exclude 了 `serve-wasm`、`integration`、`cloud_object_client`。建议 T12 完成后也 exclude `cloud_objects`、`cloud_object_models`、`warp_graphql_schema`，从源头阻止它们参与编译。

3. **系统提示词外部化**: 交接文档"下一步建议"第 4 条 — 将 `include_str!` 改为运行时文件读取，减小 binary 体积并允许用户自定义提示词。

---

## 附：已有瘦身工作回顾

以下工作已完成（来自 SLIM_PLAN_V2.md / TODO_REMAINING.md），此处仅作记录：

| 任务 | 内容 | 状态 |
|------|------|------|
| T9 | ID 类型 (ClientId/SyncId/ServerId 等) → warp_types | ✅ |
| T10 | Auth 类型 (UserUid/UserMetadata 等) → warp_types | ✅ |
| T11 | http_client warp-cloud feature split (IAP/headers optional) | ✅ |
| T13 | strip = "debuginfo" 添加到 release profile | ✅ |
| T14 | 14 个 Channel::Oss 单元测试 | ✅ |
| T16 | 删除 warp_server_client 幽灵 session-sharing-protocol 依赖 | ✅ |
| T17 | warp-workflows optional (bundled_workflows feature) | ✅ |
| T18 | warp_isolation_platform 中 warp_core optional (env var fallback) | ✅ |
| T19 | 14 云 features 整合为 cloud_ui feature group | ✅ |
| T20 | warp_assets slim feature (exclude 41MB async/) | ✅ |
| T21 | websocket tls/proxy/graphql feature split | ✅ |
| Wave 0-4 | 删除纯云 crate + app 层云模块 + 迁移替换 + 删除 server/ | ✅ |
| Issue #21 | Zap→Marb 品牌重命名 + 非 ASCII 输入检测 + todo!() panic 修复 | ✅ |
| Issue #22 | BYOP Agentic Loop (websearch/webfetch/todowrite 迭代) | ✅ |

**未完成的已有计划项**：

| 任务 | 内容 | 状态 |
|------|------|------|
| T12 | 196 文件 warp_graphql import gating (5 阶段) | ❌ 待做 |
| T15 | BYOP 集成测试 | ❌ 待做 |
