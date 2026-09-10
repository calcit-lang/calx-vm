# Distinguish strict Tag from Str / 区分严格 Tag 与 Str

## English

- Added concrete `Calx::Tag` and appended `CalxType::Tag` without renumbering existing encoded variants.
- Split Cirru `|text` and `:tag` parsing, and retained their prefixes in Display/CLI output.
- Admitted Tag through the existing strict program, validator, builder, named-entry, storage, control-flow, and typed-host paths.
- Added parser/round-trip, discriminant, mismatch, runtime, builder, CLI/demo, and host-boundary regressions.
- Documented the pre-1.0 0.6.0 candidate scope and explicit non-goals; formal Calcit adoption waits for publication.

## 中文

- 增加具体 `Calx::Tag`，并在不重排既有编码 variants 的前提下追加 `CalxType::Tag`。
- 分离 Cirru `|text` 与 `:tag` 解析，Display/CLI 输出保留各自前缀。
- 让 Tag 复用现有 strict program、validator、builder、具名入口、存储、控制流与 typed host 通道。
- 增加 parser/round-trip、discriminant、错配、runtime、builder、CLI/demo 与 host boundary 回归。
- 记录 pre-1.0 0.6.0 候选范围与明确非目标；Calcit 正式采用等待发布。
