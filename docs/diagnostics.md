# Calx 源码位置与诊断边界 / Source-aware diagnostics

Calx 保留 parse、validation、runtime、host 四个错误所有者，同时通过
`DiagnosticView` 提供共同的只读投影。这样 CLI 和工具可以读取统一字段，但不会把
解析器、验证器、VM snapshot 与宿主状态合并成一个大错误对象。

| 阶段 / Phase | 稳定代码 / Stable code | 错误类型 | 上下文 |
| --- | --- | --- | --- |
| Parse | `CALX_PARSE_CIRRU` | `ParseError` | Cirru parser position |
| Parse | `CALX_PARSE_INSTRUCTION` | `ParseError` | function, expanded instruction index, source span |
| Validation | `CALX_VALIDATION` | `ValidationError` | function, instruction, source span, expected/actual typed stack |
| Runtime | `CALX_RUNTIME_TRAP` | `CalxError` | function, instruction, source span, boxed VM snapshot |
| Host | `CALX_HOST_IMPORT` | `CalxError` | message only; no fabricated VM snapshot |

`SourcePosition` 的 line/column 从 1 开始，offset 是从 0 开始的 byte offset；
`SourceSpan` 是半开区间。文本位置统一显示为 `file:line:column`。稳定代码表示诊断类别，
message 的具体措辞仍可继续改进。

## 带位置的解析 / Source-aware parsing

需要源码位置时使用 `parse_program`：

```rust
use calx_vm::parse_program;

let program = parse_program("demo.cirru", "fn main ()\n  nop")?;
let first_span = program.functions[0].source_spans[0].as_ref();
# Ok::<(), calx_vm::ParseError>(())
```

Cirru AST 目前不保存节点位置。Calx 会将带位置的语义 token 与 Cirru 已解析、已处理
`$`/`,` 的 AST 确定性对齐，再镜像 folded call 与结构化控制流的展开顺序。最终
`CalxFunc::source_spans` 与 `syntax` 一一对应，lowering 不改变该索引关系。

旧入口 `parse_function(&[Cirru]) -> Result<CalxFunc, String>` 继续保留，供只有 AST 的
调用方使用；它生成空的 `source_spans`，验证和执行仍正常，只是诊断不显示源码位置。

## ValidationError

`ValidationError` 包含稳定代码、函数名、扁平 syntax index、原因、可选 source span，
以及实际 typed operand stack。类型不匹配时还提供可选 expected stack。较大的可选字段
放在堆上，避免每个 `Result` 的 inline error 膨胀。验证阶段没有运行 VM，不附加虚构的
运行栈或 frame。

## CalxError 与按需 snapshot

`CalxError` 的 inline 数据仍只有 message 和可选 boxed snapshot：

```rust
pub struct CalxError {
  pub message: String,
  pub snapshot: Option<Box<CalxErrorSnapshot>>,
}
```

VM 内部 trap 的 code、source span、operand stack、top frame 和 globals 都存放在已有的
`CalxErrorSnapshot` 中，只在错误路径分配。宿主 import 可用 `CalxError::new_raw(message)`
报错；这类错误使用 `CALX_HOST_IMPORT`，且 `snapshot`、function、instruction、span 都为
`None`，不会伪造 VM 上下文。

## 兼容性与迁移 / Compatibility and migration

- `CalxFunc` 新增公开字段 `source_spans`。手工构造 synthetic function 的代码需要增加
  `source_spans: Rc::new(vec![])`，或提供与 `syntax` 等长的 span 表。
- 直接依赖的 `cirru_parser` 升级至 0.2.15。直接调用 `cirru_parser::parse` 的下游代码会
  收到 `CirruError` 而不再是 `String`；如需旧行为可调用 `error.to_string()`。
- `ValidationError` 的可选 source/expected-stack payload 改为 out-of-line。工具应优先
  使用 `error.diagnostic()` 获取稳定只读视图。
- 手工构造 `CalxErrorSnapshot` 的代码需要同时提供 `code` 与 `source_span`；正常 VM
  执行路径会自动填写这两个字段。
- `Display` 输出现在以 `error[CODE] phase` 开头。此前匹配完整错误字符串的 consumer
  应迁移到 `DiagnosticCode` 和结构化字段。
- #21 把旧的 `CalxError.stack/top_frame/globals` 移入 `CalxErrorSnapshot`；accessor
  `stack()`、`top_frame()`、`globals()` 继续可用。#30 没有增大 `CalxError` inline layout。

当前承诺文本 renderer 和有界 runtime trace；JSON schema 与彩色输出仍不在这一阶段。`calx trace FILE`
复用真实 interpreter 的事件流，默认限制 10,000 个 VM transition；详情见
[有界运行时 trace](./tutorials/runtime-trace.md)。

## 大小与成功路径测量

#21 的基线测试约束 `CalxError` inline size 不超过 4 个 machine words；64-bit 目标上即
不超过 32 bytes。Criterion 对 `instruction_execution` 和 `multiple_calls` 的复测都未发现
统计显著的成功路径性能变化。#30 继续保留这一大小测试；新增 diagnostic metadata 仅进入
解析/验证错误或 runtime 的既有 boxed snapshot。
