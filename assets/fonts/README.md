# GUI 内嵌字体

构建带 `gui` feature 的 `symm` 时，**必须**存在本目录下的：

`NotoSansSC-Regular.otf`

该文件会通过 `include_bytes!` 打进二进制；**不再**扫描系统字体或读取 `SYMM_FONT_PATH`。

## 获取字体

```bash
scripts/fetch-gui-font.sh
```

下载的是 [Noto Sans SC](https://fonts.google.com/noto/specimen/Noto+Sans+SC) **Subset OTF**（约 8MB，覆盖常用简体字）。

## 图标字体

界面图标使用 **Phosphor Regular**（[`egui-phosphor`](https://crates.io/crates/egui-phosphor) 0.8，MIT），字体数据由 crate 内嵌，无需额外文件。

## 许可

- Noto Sans SC：SIL Open Font License 1.1
- Phosphor：MIT
