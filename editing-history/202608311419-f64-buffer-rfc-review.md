# 收口 F64Buffer RFC review / Close F64Buffer RFC review findings

## 中文

- parser 契约显式列出 `f64-buffer.len`、`f64.to-i64-index`、`f64-buffer.get` 三条指令，避免实现漏掉 checked conversion。
- 更新 RFC 0003 的历史顺序：#39 scalar 数据已触发 RFC 0004/#50，后续按 #51 → #52 → #53 推进，不再等待 umbrella issue 整体关闭。
- roadmap 的现有文件树只列实际存在的 RFC；linear memory 与 binary container 保留为 M4 候选，不提前占用文件名或编号。
- 确认 conversion 只验证 finite/integral/`0 <= n < 2^63`，buffer bounds 仍由 `f64-buffer.get` 唯一检查。

## English

- List all three parser instructions—`f64-buffer.len`, `f64.to-i64-index`, and `f64-buffer.get`—so the checked conversion cannot be omitted from implementation.
- Update the historical order in RFC 0003: the scalar evidence from #39 has activated RFC 0004/#50, and work now proceeds through #51 → #52 → #53 without waiting for the umbrella issue to close.
- Keep only existing RFCs in the roadmap file tree; linear memory and the binary container remain M4 candidates without preassigned filenames or numbers.
- Confirm that conversion checks only finite/integral/`0 <= n < 2^63`, while `f64-buffer.get` remains the sole buffer-bounds check.
