# symm

symm 是一个跨平台软链接管理工具，包含桌面 GUI（`symm`）和命令行工具（`symm-cli`）。它把每条链接记录写入本地 SQLite，并围绕「创建 / 纳管 / 查看 / 删除 / 恢复」这些操作处理路径迁移、占用检测、状态探测和 Windows 提权。

核心约定：

- 记录以 `link_path` 幂等 upsert；同一个链接路径再次执行 `add` / `adopt` / `point` 会更新原记录。
- `target` 是真实数据位置，`link` 是对外访问位置。
- 非空 `name` 是可读别名，在库内唯一；纯数字名称入库时会加 `link-` 前缀，避免和 `ls` 序号冲突。
- 任一步失败即停止，不做自动回滚；中间态需要人工检查后重试。

## 功能

| 能力 | GUI | CLI |
|------|-----|-----|
| 添加记录 / 创建链接 | 支持 | `symm-cli add <link> <target>` |
| 接管实体后建链 | 支持 | `symm-cli adopt <link> <target>` |
| 修改已有链接指向 | 支持 | `symm-cli point <link> <target>` |
| 列表 / 搜索 / 状态 | 支持 | 列表 / 状态：`symm-cli ls` |
| 详情 | 支持 | `symm-cli show <序号或名称>` |
| 删除链接关系 | 支持 | `symm-cli rm <序号或名称>...` |
| 恢复目标到链接位置 | 支持 | `symm-cli restore <序号或名称>...` |
| JSON 输出 | 不适用 | `ls --json` / `show --json` |

终端的人类可读输出使用简体中文；JSON 与 `--status` 参数保持英文枚举，便于脚本处理。

## 快速开始

依赖：

- Rust stable
- Git
- Windows 安装包以 GitHub Actions 的 Windows runner + Inno Setup 构建结果为准；本地不提供 installer 构建入口

首次构建 GUI 前需要下载内嵌中文字体：

```bash
scripts/fetch-gui-font.sh
```

构建 GUI 与 CLI：

```bash
mise run build
```

产物：

- `target/release/symm`：桌面 GUI
- `target/release/symm-cli`：命令行工具
- Windows 上扩展名为 `.exe`

本地开发推荐入口：

```bash
mise run run-gui
mise run run-help
mise run fmt-check
mise run clippy
mise run test
mise run test-gui
```

发布门禁以 GitHub Actions 为准；本地检查用于提前发现问题，不代替远端 CI。

## CLI 使用

```bash
symm-cli add <link> <target>
symm-cli adopt <link> <target>
symm-cli point <link> <target>
symm-cli ls [--status ok|broken|missing|stale|drift|unknown] [--json] [--limit N] [--offset N]
symm-cli show [序号或名称] [--json]
symm-cli rm [序号或名称]...
symm-cli restore [序号或名称]...
```

选择器规则：

- `show` / `rm` / `restore` 的纯数字选择器表示当前 `ls` 序号。
- 非纯数字选择器按 `name` 查找。
- `show` 省略选择器时进入交互选择。
- `rm` / `restore` 可一次传多个选择器；省略时进入交互多选。
- `ls` 表格默认每页 100 条；`--json` 默认返回全量数组，只有显式 `--limit` / `--offset` 时分页；`--limit` 必须是正整数，`--offset` 必须是非负整数。

示例：

```bash
symm-cli add ~/.config/app ~/data/app-config
symm-cli adopt ~/.config/app ~/data/app-config
symm-cli point ~/.config/app ~/data/app-config-v2
symm-cli ls --status ok
symm-cli show 1
symm-cli rm 1
symm-cli restore app-config
```

## 状态与类型

| JSON / `--status` | 终端显示 | 含义 |
|-------------------|----------|------|
| `ok` | 正常 | 链接存在，目标存在，指向与数据库一致 |
| `broken` | 目标没了 | 链接仍指向数据库中的 target，但 target 路径不存在 |
| `missing` | 链接没了 | 链接路径不存在 |
| `stale` | 链接已陈旧 | link 路径存在，但不是记录期望的链接实体，或链接类型与记录不一致 |
| `drift` | 指向不对 | 链接仍存在，但已经指向数据库记录以外的位置 |
| `unknown` | 未知 | 权限、I/O 或读取 link 指向失败，无法可靠判断状态 |

| JSON 类型 | 终端显示 |
|-----------|----------|
| `symlink` | 软链接 |
| `junction` | 目录联接 |

## 数据目录

默认数据目录是可执行文件同级的 `data/`：

```text
symm/
  symm
  cli/
    symm-cli
  data/
    symm.db
    settings.json
```

特殊规则：

- 当 CLI 位于 `cli/` 子目录时，`symm-cli` 会自动使用上一级应用根目录的 `data/`，与 GUI 共用数据库。
- `symm.db` 存链接记录。
- 默认 `data/settings.json` 存 GUI 偏好，和可切换的链接库数据目录分离。
- 设置 `SYMM_HOME` 后，链接数据库使用该目录；GUI 偏好仍写入默认 `data/settings.json`，避免自定义数据目录重启后丢失。

```bash
SYMM_HOME=/var/lib/symm symm-cli ls
```

## 环境变量

CLI 默认在需要决策时弹出终端交互菜单。下列变量用于跳过对应菜单，适合脚本或自动化场景。变量值不合法时会直接报错，不会静默回退到菜单。

| 变量 | 用途 |
|------|------|
| `SYMM_HOME` | 指定数据目录 |
| `SYMM_LINK_OP_LINK` | `add` / `adopt` / `point` 未传 link 位置参数时指定 link 路径 |
| `SYMM_LINK_OP_TARGET` | `add` / `adopt` / `point` 未传 target 位置参数时指定 target 路径 |
| `SYMM_LINK_OP_NAME` | `add` / `adopt` / `point` 写库前指定记录名称 |
| `SYMM_LINK_OP_LOCK_CHOICE` | 链接操作遇到 link 路径被占用时选择是否解除占用 |
| `SYMM_PERF_LOG` | 在 stderr 输出 workflow 耗时 |

### `SYMM_LINK_OP_NAME`

```bash
SYMM_LINK_OP_NAME=my-project symm-cli add ./link ./target
```

空名称允许多条；非空名称必须唯一。

### `SYMM_LINK_OP_LOCK_CHOICE`

| 取值 | 效果 |
|------|------|
| `unlock` / `kill` / `continue` | 结束占用进程，等待句柄释放后继续 |
| `cancel` / `abort` | 不结束进程，取消本次链接操作 |

### `SYMM_PERF_LOG`

设为 `1` 或非 `0` / `false` / `no` 的值即开启。

```bash
SYMM_PERF_LOG=1 symm-cli ls
```

## GUI

GUI 使用 `eframe` / `egui`，通过 `gui` feature 构建。它复用 CLI 的业务 workflow，不维护另一套链接逻辑。

主要能力：

- 侧栏搜索、状态刷新、详情查看。
- GUI 列表选择和批量操作使用 record id，不显示也不依赖 CLI 的全库 `ls` 序号。
- 链接操作对话框中显式选择创建链接、接管实体或改指向。
- GUI 默认不结束占用进程；只有先展示占用进程列表，并由用户再次确认后，才会结束占用进程并继续链接操作。
- 批量删除链接关系，也可单独执行恢复目标到链接位置。
- 设置明暗模式、配色、字号、侧栏宽度和数据目录。
- 内嵌 Noto Sans SC 和 Phosphor 图标字体，不依赖系统字体。

字体文件要求见 `assets/fonts/README.md`。CI 和打包 workflow 会自动运行 `.github/actions/fetch-gui-font`。

## 平台行为

| 能力 | Linux / macOS | Windows |
|------|---------------|---------|
| 建链 | `symlink` | 优先软链接；目录软链失败时可降级为 junction |
| 同盘判断 | `dev` | 盘符 |
| 查占用 | `fuser` / `lsof`；需要时走 sudo 子进程 | Restart Manager；非管理员时走 UAC 子进程 |
| 结束占用 | sudo 子进程 | UAC 子进程 + `TerminateProcess` |
| 建链提权 | 无 | 普通建链失败且需要提权时走 UAC |
| 跨盘目录 ACL | 不适用 | `icacls` 快照，失败则跳过恢复 |
| 同盘迁移软链 | `rename` + 树内 rebase | `rename`；拒绝访问时重建链接 |

交互式终端下 Linux / macOS 的 `sudo` 可输入密码；无 TTY 自动化环境可能失败。

Windows 占用检测说明：

- 主进程保持当前用户身份，迁移、复制和写库不在主进程提权。
- 非管理员查占用 / 结束占用时，使用 `runas` 启动隐藏窗口的内部子命令。
- Restart Manager 扫描的是迁移目录下普通文件清单，分批注册资源；遇到不可访问路径会拆分跳过。
- 提权子进程的结果通过临时文件回传，扫锁进度在主终端显示。
- 杀进程后只短暂等待句柄释放，不重复二次 UAC 扫描。
- 建链提权独立于扫锁策略，仅在普通建链失败且错误需要提权时触发。

## 链接操作流程

`add` 只创建或登记 link 指向已存在 target：

| 场景 | 行为 |
|------|------|
| `link` 不存在且 `target` 存在 | 创建链接并写库 |
| `link` 已是链接且指向同一 `target` | 不重建链接，只写库或更新记录 |
| `link` 已是链接但指向别处 | 报错，提示使用 `point` |
| `link` 是真实文件或目录 | 报错，提示使用 `adopt` |
| `target` 不存在 | 报错 |

`adopt` 接管 link 路径上的真实实体：

| 场景 | 行为 |
|------|------|
| `link` 是真实文件或目录且 `target` 不存在 | 将 link 实体迁到 target，再在 link 原位置建链并写库 |
| `link` 已是链接 | 报错，提示使用 `add` 或 `point` |
| `target` 已存在 | 报错，不替换 target |

`point` 修改已有链接指向：

| 场景 | 行为 |
|------|------|
| `link` 已是链接且 `target` 存在 | 先创建临时 link 指向新 target，再替换当前 link 并写库 |
| `link` 不是链接 | 报错 |
| `target` 不存在 | 报错 |

三个操作都会先执行便宜 preflight；能在文件系统变更前发现的 name 冲突会直接失败，不创建 link、不迁移实体、不改指向。需要改动 link 路径时会检查占用，按 `SYMM_LINK_OP_LOCK_CHOICE` 解除或取消；最终都会以 `link_path` upsert 数据库记录。

| 场景 | 行为 |
|------|------|
| 写库失败 | 可能已经完成文件系统变更但没有记录，需要人工对齐 |

## `rm` / `restore` 流程

`rm` 停止管理链接关系，不移动 target：

| 状态 | 行为 |
|------|------|
| `ok` / `broken` | 删除 link，删除数据库记录 |
| `missing` | 只删除数据库记录 |
| `stale` | 不碰 link 路径上的真实实体，只删除数据库记录 |
| `drift` | 不碰当前 link，只删除数据库记录 |
| `unknown` | 报错并保留记录，不把探测失败当作 missing |

`restore` 把 target 迁回 link 路径，然后删除记录：

| 状态 | 行为 |
|------|------|
| `ok` | 删除当前 link，把记录里的 target 迁回 link，删除数据库记录 |
| `missing` | target 存在时仍尝试迁回 link，删除数据库记录 |
| `broken` | 报错并保留记录 |
| `stale` | 报错并保留记录，不覆盖 link 路径上的真实实体 |
| `drift` | 报错并保留记录，不覆盖当前 link |
| `unknown` | 报错并保留记录，不移动 target、不删除 link |

`restore` 复用迁移能力；目录内部链接会保持为链接，指向被迁移目录内部的链接会 rebase 到新位置。

## 迁移与 rebase

- 同盘迁移使用 `rename`，随后对目标树做单遍 rebase。
- 跨盘迁移使用复制，进度按已复制字节和已处理文件数输出。
- 跨盘目录复制保持文件内容流式处理；为恢复目录权限和重建内部链接，允许暂存目录权限与内部链接元数据，不要求严格 O(目录深度) 内存。
- 树内绝对路径软链会改写到新根。
- 相对路径软链通常不改写。
- 指向树外的链接保持原目标。
- 跨盘删源失败时，目标可能已存在且源仍在，错误信息会说明需要人工清理。

占用扫描只能发现平台 API 能报告的资源；迁移阶段仍可能遇到文件锁，需要退出相关程序后重试。

## 代码结构

```text
src/
  bin/
    symm.rs                      # GUI 入口
    symm-cli.rs                  # CLI 入口；内部 __elevated-* 子命令在此分流
  app/
    dispatch.rs                  # 命令分发，只把 CLI command 交给 workflow
  domain/
    model.rs                     # LinkRecord / LinkView / LinkStatus / LinkKind / name 规则
    error.rs                     # SymmError
    gui_settings.rs              # GUI 偏好模型
  workflows/
     link_ops/                    # add / adopt / point 主流程、路径输入、占用 gate
    rm/                          # rm / restore 主流程
    ls/                          # ls 输出流程
    show/                        # show 输出流程
    list_views.rs                # 从记录构造带状态的 LinkView
    pick_list.rs                 # 交互选择列表数据
    select.rs                    # 无 selector 时交互选择
    selector.rs                  # 序号 / name 解析
    perf.rs                      # SYMM_PERF_LOG
  adapters/
    db/                          # SQLite schema、query、repository
    paths/                       # SYMM_HOME、路径规范化、remove、rebase 路径计算
    status/                      # 读盘探测 ok/broken/missing/stale/drift
    symlink/                     # 建链、写链、删链；Windows 策略在 windows.rs
    migrate/                     # rename / copy / rebase 编排
    lock/                        # 占用检测、解除占用、提权子进程协议
    platform/                    # OS API、提权、host fs、进程 API
    errors/                      # IO 错误映射
    settings.rs                  # 通用设置辅助
  gui/
    app.rs                       # egui 应用主体与后台任务
    data.rs                      # GUI 调 workflow 的数据入口
    state.rs                     # GUI 状态、快照、筛选缓存
    panels/                      # 顶栏、侧栏、内容、添加、删除、设置
    widgets/                     # 项目内 GUI 控件
    theme/                       # 字体、配色、排版
  ui/
    cli.rs                       # clap 命令定义
    output.rs                    # 表格 / JSON / 错误 JSON
    interaction/                 # inquire 交互
    progress/                    # 迁移进度输出
```

依赖方向：

```text
bin / app
  -> workflows
  -> adapters::{db, paths, status, symlink, migrate, lock}
  -> adapters::platform
  -> domain

gui -> workflows / adapters::db / domain
ui  -> domain
```

分层约束：

| 位置 | 约束 |
|------|------|
| `workflows/**` | 不写平台 `cfg`，不直接依赖 `adapters::platform`，只编排业务动作 |
| `adapters/migrate/**` | 不新增平台分支；写链只走 `adapters::symlink` |
| `adapters/symlink/**` | Windows 专用策略集中在 `windows.rs` |
| `adapters/status/**` | 只负责读盘状态探测 |
| `adapters/platform/**` | 集中 OS API、提权、host fs 和进程能力 |
| `app/**` | 只做分发，不放业务 helper |
| `ui/**` | 只做命令定义、输出、交互和进度展示 |

常用入口：

| 能力 | 入口 |
|------|------|
| 打开数据库 / CRUD | `adapters::db::repository` |
| 查询条件 | `adapters::db::query::LinkQuery` |
| 链状态 | `adapters::status::{for_record, to_view}` |
| 建链 / 写链 / 删链 | `adapters::symlink::{create_link, write_symlink, unlink}` |
| 迁移 | `adapters::migrate::{migrate_path, move_path_with_retry}` |
| OS 文件系统能力 | `adapters::platform::host_platform()` |
| 占用检测 / 解除 | `adapters::lock::*` |

## 数据库

SQLite 连接参数：

- `busy_timeout = 5000`
- `journal_mode = WAL`
- `synchronous = NORMAL`
- `temp_store = MEMORY`

主表 `links`：

| 字段 | 说明 |
|------|------|
| `id` | 自增主键 |
| `name` | 可读别名，空字符串表示未命名 |
| `link_path` | 链接路径，唯一 |
| `target_path` | 目标路径 |
| `link_kind` | `symlink` 或 `junction` |
| `created_at` / `updated_at` | 时间戳 |

索引：

- `ux_links_link_path`：`link_path` 唯一。
- `ux_links_name_nonempty`：非空 `name` 唯一。

## 打包

便携包布局：

```text
symm/
  symm              # GUI
  cli/
    symm-cli        # CLI
  data/             # 默认数据目录
```

Windows 安装包目前只为 x64 构建；Windows arm64 / x86 提供便携 zip。Linux 和 macOS 均提供便携 zip。

发布产物：

| 平台 | 架构 | 产物 |
|------|------|------|
| Windows | x64 | `symm-setup-windows-x64.exe`、`symm-portable-windows-x64.zip` |
| Windows | arm64 | `symm-portable-windows-arm64.zip` |
| Windows | x86 | `symm-portable-windows-x86.zip` |
| Linux | x64 | `symm-portable-linux-x64.zip` |
| Linux | arm64 | `symm-portable-linux-arm64.zip` |
| macOS | x64 | `symm-portable-macos-x64.zip` |
| macOS | arm64 | `symm-portable-macos-arm64.zip` |
| 全平台 | 全架构 | `SHA256SUMS` |

## GitHub Actions

| Workflow | 触发 | 说明 |
|----------|------|------|
| `ci.yml` | 影响代码、测试、脚本、打包、assets 或 workflow 的 push / PR | 多平台矩阵执行 fmt、clippy、test |
| `workflow-lint.yml` | workflow 或 action 变化的 push / PR | 运行 actionlint，检查 GitHub Actions 配置 |
| `build-release-assets.yml` | `workflow_call` | 正式发布和测试包共用的平台构建流程 |
| `release.yml` | `vX.Y.Z`，无 `-` 后缀 | 正式 Release，全平台全架构，设为 Latest |
| `release-test.yml` | `vX.Y.Z-test*` tag 或手动触发 | Pre-release，不设为 Latest，可按目标包控制 |
| `cleanup-test-releases.yml` | 定时或手动 | 清理旧测试 Pre-release，只保留最新测试包 |

发布 workflow 不重复跑测试；它们先用 `.github/actions/verify-ci-passed` 校验当前 commit 的 `ci.yml` 已成功。发布 tag 不应指向只修改 README、ADR、AGENTS 或本地工具配置的维护 commit；这类文档-only 变更不触发打包所需的 CI，发布 tag 应指向已有成功 CI 的代码、脚本、打包、assets 或 workflow 相关 commit。若该 commit 修改了 workflow 或 action，还会校验 `workflow-lint.yml` 已成功。

Runner 矩阵：

| 目标 | Runner |
|------|--------|
| Linux x64 | `ubuntu-24.04` |
| Linux arm64 | `ubuntu-24.04-arm` |
| Windows x64 | `windows-2025-vs2026` |
| Windows arm64 | `windows-11-arm` |
| Windows x86 | `windows-2025-vs2026` + `i686-pc-windows-msvc` |
| macOS x64 | `macos-15-intel` |
| macOS arm64 | `macos-15` |

正式发布流程：

1. 推送 `vX.Y.Z` tag。
2. 校验该 commit 的 CI 已通过。
3. 解析版本号并准备 Release 正文。
4. 各平台构建并上传临时 artifact。
5. 发布 job 下载全部 artifact，生成统一 `SHA256SUMS`，创建 / 更新正式 Release。

测试包 tag：

| Tag | 构建目标 |
|-----|----------|
| `vX.Y.Z-test` | 全平台全架构 |
| `vX.Y.Z-test-windows` / `vX.Y.Z-test-win` | Windows 全架构 |
| `vX.Y.Z-test-linux` | Linux 全架构 |
| `vX.Y.Z-test-macos` / `vX.Y.Z-test-mac` | macOS 全架构 |
| `vX.Y.Z-test-windows-linux` | 多平台组合 |

禁止在 `test` 后追加数字，例如不要使用 `v0.1.0-test15`。

手动测试包输入：

| 输入 | 默认 | 目标产物 |
|------|------|----------|
| `build_windows_x64` | `true` | Windows x64 安装包 + 便携包 |
| `build_windows_arm64` | `false` | Windows arm64 便携包 |
| `build_windows_x86` | `false` | Windows x86 便携包 |
| `build_linux_x64` | `true` | Linux x64 便携包 |
| `build_linux_arm64` | `false` | Linux arm64 便携包 |
| `build_macos_x64` | `true` | macOS x64 便携包 |
| `build_macos_arm64` | `false` | macOS arm64 便携包 |
| `release_notes` | 空 | 可选，覆盖测试 Release 正文 |

手动触发默认只构建 `windows-x64`、`linux-x64`、`macos-x64`。

```bash
# 只打 Windows x64
gh workflow run release-test.yml \
  -f build_windows_x64=true \
  -f build_linux_x64=false \
  -f build_macos_x64=false

# 只打 Linux arm64
gh workflow run release-test.yml \
  -f build_windows_x64=false \
  -f build_linux_x64=false \
  -f build_macos_x64=false \
  -f build_linux_arm64=true
```

手动触发没有 semver tag，Release tag 会使用 `test-run-<run_id>`；版本号来自当前 `Cargo.toml`。

## 维护约定

- 改 Rust 代码后优先跑 `mise run fmt-check`、`mise run clippy`、`mise run test` 和 `mise run test-gui`；提交前优先跑 `mise run ci`。
- 发布和测试包以 GitHub Actions 结果为准，不以本地 `target/release/*` 作为交付物。
- 改 workflow 或 action 时，远端 `workflow-lint.yml` 必须通过；本机没有 `actionlint` 时不要声称已跑。
- 修改分层相关代码时，保持 `workflows` 无平台分支，平台差异集中在 adapters。
