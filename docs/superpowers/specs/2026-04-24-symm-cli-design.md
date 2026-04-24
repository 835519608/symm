# Symm CLI 设计说明（MVP）

## 目标

实现一个高性能、跨平台的软链接管理 CLI，支持创建、删除、查看。

## MVP 范围

- 仅提供 CLI。
- 仅管理由本工具创建的链接。
- Windows 下目录链接创建失败时，从 symlink 自动降级为 junction。
- 默认人类可读输出，同时支持 `--json`。

## 命令

- `symm add <name> <target> <link>`
- `symm rm <name|link>`
- `symm ls [--json] [--status ok|broken|missing]`
- `symm show <name|link> [--json]`

## 数据模型

- 默认数据目录：可执行文件同级 `data/`
- 可通过 `SYMM_HOME` 覆盖
- 数据库文件：`symm.db`
- 表：`links(name, link_path, target_path, link_kind, created_at, updated_at)`
- 唯一索引：`name`、`link_path`

## 状态模型（运行时计算）

- `ok`：链接与目标都存在
- `broken`：链接存在但目标不存在
- `missing`：数据库有记录但链接对象不存在

## 一致性策略

- `add`：先建链接，再写库；写库失败时回滚删除链接
- `rm`：先查受管对象，删除链接，再删除数据库记录
- 所有写操作通过事务保证一致性

## 默认接管行为（add）

当执行 `symm add <name> <target> <link>` 且满足：
- `link` 路径已存在且为实体（文件/目录），并且不是软链接
- `target` 路径不存在

工具默认执行“接管迁移”：
1. 原子重命名 `link -> link.__symm_staging__`（失败通常为占用/权限）
2. 原子重命名 `staging -> target`
3. 在原 `link` 位置创建软链接指向 `target`
4. 写入 SQLite 受管记录

任一步失败都会回滚，保证不破坏原始数据。

### 占用进程处理（可选交互）

当 `link -> staging` 失败时：
- Windows：通过系统 Restart Manager API 获取占用进程 PID 列表
- Linux/macOS：尽量使用 `fuser`/`lsof` 获取 PID（若不可用则提示无法定位）

随后提供类似 pnpm 的多选界面（空格选择、回车确认）让用户选择要结束的进程；结束后重试迁移。

## 跨平台策略

- Linux/macOS：使用原生 symlink
- Windows：优先 symlink，目录失败时回退 junction，并记录实际类型

## 验收标准

- 能完成 add/rm/ls/show 全流程
- 支持重启后数据持久化
- 状态 `ok|broken|missing` 判断正确
- Windows 目录链接可通过 symlink 或 junction 成功创建
- `ls`、`show` 同时支持表格与 `--json`

