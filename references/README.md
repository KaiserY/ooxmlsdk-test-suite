# Office 格式规范

本目录统一保存 Office 格式开发所需的 Microsoft Open Specifications。实现
`olecfsdk` 时，以这里的官方 DOCX 为格式定义和字段合法性的主要事实来源；
LibreOffice、Apache POI 以及 corpus 用于交叉验证兼容行为和真实文件边界。

DOCX 是权威原件。Markdown 仅用于全文搜索和日常阅读；遇到转换歧义、复杂表格、
公式或图示时，必须回到对应 DOCX 核对。

## 转换为 Markdown

在本目录执行以下命令。每份规范使用独立的 media 子目录，避免不同文档中同名图片
互相覆盖；`raw_html` 用于尽量保留 GFM 无法表达的复杂表格等结构。

```bash
stem='[MS-DOC]-260217'

pandoc "${stem}.docx" \
  --from=docx \
  --to=gfm+footnotes+raw_html \
  --extract-media="./media/${stem}" \
  --wrap=none \
  -o "${stem}.md"
```

转换其他规范时只需修改 `stem`。生成后不要手工简化 raw HTML 表格；若 DOCX 更新，
直接重新生成对应 Markdown 和 media 目录。
