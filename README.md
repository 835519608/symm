# symm

高性能、跨平台的软链接管理命令行工具。

## 命令

- `symm add <name> <target> <link>`：创建并登记软链接
- `symm rm <name|link>`：按名称或链接路径删除
- `symm ls [--status ok|broken|missing] [--json]`：列表查看
- `symm show <name|link> [--json]`：查看单条详情

## 数据目录

- 默认目录：`$HOME/.symm`
- 可通过 `SYMM_HOME` 覆盖
- 注册库文件：`symm.db`

## 平台行为

- Linux/macOS：使用系统软链接
- Windows：优先创建软链接；目录软链接失败时自动降级为 junction
