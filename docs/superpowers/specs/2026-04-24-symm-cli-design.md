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

- 默认数据目录：`$HOME/.symm`
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

## 跨平台策略

- Linux/macOS：使用原生 symlink
- Windows：优先 symlink，目录失败时回退 junction，并记录实际类型

## 验收标准

- 能完成 add/rm/ls/show 全流程
- 支持重启后数据持久化
- 状态 `ok|broken|missing` 判断正确
- Windows 目录链接可通过 symlink 或 junction 成功创建
- `ls`、`show` 同时支持表格与 `--json`

