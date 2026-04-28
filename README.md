# symm

高性能、跨平台的软链接管理命令行工具。

## 项目介绍

- 管理软链接生命周期：创建、更新、查看、删除
- `add` 支持冲突分支处理、占用探测、回滚保护
- 跨平台支持 Linux / macOS / Windows（Windows 目录支持 junction 回退）
- 持久化使用 SQLite，按 `link_path` 幂等 upsert

## 快速开始

### 1) 前置依赖

- Rust stable（建议通过 `rustup` 安装，含 `cargo`）
- Git
- 平台：
  - Windows 11
  - Linux
  - macOS
- Windows 本地构建额外需要：
  - Visual Studio Build Tools（或 Visual Studio）中的 C++ 构建工具链（提供 `link.exe`）

### 2) 构建

- `cargo build --release`

### 3) 常用命令

- 添加或纳管链接：`symm add <link> <target>`
- 查看全部：`symm ls`
- 查看单条：`symm show <name|link>`
- 删除记录与链接：`symm rm <name|link>`

### 4) 测试与质量检查

- 格式检查：`cargo fmt --all -- --check`
- Clippy：`cargo clippy --all-targets --all-features -- -D warnings`
- 测试：`cargo test --all-targets`

## 命令说明

- `symm add <link> <target>`：创建/更新软链接（按 link 幂等）
- `symm rm <name|link>`：按名称或链接路径删除
- `symm ls [--status ok|broken|missing] [--json]`：列表查看
- `symm show <name|link> [--json]`：查看单条详情

## 数据目录

- 默认目录：可执行文件同级的 `data/` 目录
- 可通过 `SYMM_HOME` 覆盖
- 注册库文件：`symm.db`

## 当前架构（分层 + 单一职责）

```text
src/
  bin/
    symm.rs                     # CLI 入口
  app/
    service.rs                  # 命令分发
    commands/
      add.rs                    # add 编排
      rm.rs
      ls.rs
      show.rs
  domain/
    error.rs                    # 领域错误
    model.rs                    # 领域模型
  usecases/
    add/
      adopt.rs                  # add 冲突策略与回滚
      ports.rs                  # 用例层端口（trait）
  infra/
    db/repository.rs            # SQLite 读写
    fs/                         # 链接/迁移/ACL/路径操作
    paths/                      # 目录与路径规范化
    platform/admin.rs           # 平台权限能力
    processes/                  # 占用探测与进程终止
    errors/io_map.rs            # IO 错误映射
  interface/
    cli.rs                      # 参数模型
    output.rs                   # 输出渲染
    interaction/choice.rs       # 交互与 env 选择
    progress/migration_reporter.rs
```

### 依赖方向

```mermaid
flowchart LR
binSymm[bin/symm.rs] --> interfaceLayer[interface]
binSymm --> appLayer[app]
appLayer --> usecasesLayer[usecases]
appLayer --> infraLayer[infra]
appLayer --> domainLayer[domain]
usecasesLayer --> domainLayer
usecasesLayer --> infraLayer
infraLayer --> domainLayer
```

### 运行主流程

```mermaid
flowchart LR
cli[CLI Parse] --> guard[Windows Admin Guard]
guard --> dispatch[app/service Dispatch]
dispatch --> addCmd[commands/add]
dispatch --> lsCmd[commands/ls]
dispatch --> showCmd[commands/show]
dispatch --> rmCmd[commands/rm]
addCmd --> adoptFlow[usecases/add/adopt]
addCmd --> infraOps[infra fs/db/processes]
lsCmd --> dbRead[infra db]
showCmd --> dbRead
rmCmd --> infraOps
```

## 平台行为

- Linux/macOS：使用系统软链接
- Windows：优先创建软链接；目录软链接失败时自动降级为 junction
- Windows：程序要求管理员权限运行（UAC），用于稳定处理链接与占用场景

## `add` 行为与冲突处理

当执行 `symm add <link> <target>` 时：

- 以 `link` 为主键：同一 `link` 重复执行会更新原记录（不是新增）
- 成功后会提示可选填写 `name`：
  - 新增时默认空
  - 更新时默认显示原值，回车保持原样

- 若 `target` 不存在且 `link` 为实体（非软链接）：执行接管迁移（将 `link` 实体迁移到 `target`，再在 `link` 创建指向 `target` 的链接）
  - 同盘时优先快速移动（`rename`），通常几乎瞬时完成
  - 跨盘时自动改为复制到 `target` 后删除源路径
  - 迁移期间会持续输出阶段状态；跨盘复制时会显示已复制大小进度
- 若 `target` 与 `link` 都存在：进入三选一交互
  - 保留 `link`（放弃 `target`）
  - 保留 `target`（放弃 `link`）
  - 取消
- 若 `target` 与 `link` 都存在且 `link` 已是软链接：
  - 若已指向 `target`：直接纳入/更新数据库记录（不再做冲突选择）
  - 若指向其他位置：可选择改为指向新的 `target` 或取消
- 若 `target` 与 `link` 都不存在：返回错误，不自动创建空目标

以上流程均采用 staging + 回滚机制，任一步失败会恢复到操作前状态，避免部分成功导致的数据破坏。

### `add` 执行流程图

```mermaid
flowchart TD
start[add link target] --> lockGate[检测 link 占用]
lockGate -->|占用| lockChoice[解除占用或取消]
lockGate -->|未占用| conflict[冲突解析 adopt]
lockChoice -->|取消| failCancel[返回错误]
lockChoice -->|解除成功| conflict
conflict --> migrate[同盘 rename 或跨盘 copy+delete]
migrate --> createLink[创建 symlink/junction]
createLink --> upsert[按 link_path upsert DB]
upsert --> done[完成]
upsert -->|失败| rollback[回滚 staging 与 link]
```

## 打包与发布（多平台）

### Windows

- 构建：`cargo build --release`
- 产物：`target/release/symm.exe`
- 分发：复制 `symm.exe` 到任意目录
- 建议：将该目录加入 `PATH`，可在任意终端直接执行 `symm`

### Linux

- 构建：`cargo build --release`
- 产物：`target/release/symm`
- 可选安装：
  - `install -m 755 target/release/symm /usr/local/bin/symm`
  - 或复制到 `~/.local/bin` 并确保该目录在 `PATH`

### macOS

- 构建：`cargo build --release`
- 产物：`target/release/symm`
- 可选安装：
  - `install -m 755 target/release/symm /usr/local/bin/symm`
  - 或复制到 `~/.local/bin` 并确保该目录在 `PATH`

### 跨平台交叉编译示例（可选）

- 安装目标：`rustup target add x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-pc-windows-msvc`
- 构建指定目标：
  - `cargo build --release --target x86_64-unknown-linux-gnu`
  - `cargo build --release --target aarch64-apple-darwin`
  - `cargo build --release --target x86_64-pc-windows-msvc`

## 性能说明（当前实现）

- `ls` 与 `show` 走 SQLite 索引查询，不做目录递归扫描。
- `ls`（表格与 `--json`）采用流式输出，避免大结果集一次性占用内存。
- 状态计算基于 `symlink_metadata` 与目标存在性判定，避免断链误判。
- `add` 接管迁移时：
  - 同盘路径优先 `rename`，保留最快路径
  - 跨盘路径使用带进度回调的复制流程，避免终端长时间无反馈
- SQLite 连接默认启用：
  - `busy_timeout=5000`
  - `journal_mode=WAL`
  - `synchronous=NORMAL`
  - `temp_store=MEMORY`

## GitHub Actions

- `CI`（`.github/workflows/ci.yml`）
  - 触发：`push` 与 `pull_request`
  - 在 Linux / Windows / macOS 执行：
    - `cargo fmt --all -- --check`
    - `cargo clippy --all-targets --all-features -- -D warnings`
    - `cargo test --all-targets`
- `Release`（`.github/workflows/release.yml`）
  - 触发：推送 tag（如 `v0.2.0-test7` 或 `v1.0.0`）
  - 自动构建三平台 release 二进制并上传到 GitHub Release
