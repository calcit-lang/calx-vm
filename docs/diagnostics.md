# Calx 错误与诊断边界

Calx 把失败按发生阶段分开，避免用一个包含所有状态的大对象表示不同问题：

```text
Cirru text
  -> parse error: String
CalxSyntax
  -> ValidationError
CalxInstr execution
  -> CalxError
```

## ValidationError

`ValidationError` 表示执行前可证明的错误，包含：

- 函数名；
- 函数内扁平 syntax index；
- 原因；
- 失败位置的 typed operand stack。

验证阶段没有运行 VM，因此不得附加虚构的运行栈或 frame。`calx check` 和 `calx explain` 直接显示此错误。

## CalxError 与按需 snapshot

`CalxError` 的 inline 数据只有：

```rust
pub struct CalxError {
  pub message: String,
  pub snapshot: Option<Box<CalxErrorSnapshot>>,
}
```

VM 内部 trap 通过 `CalxErrorSnapshot` 保留当时的 operand stack、top frame 和 globals。快照在错误路径上分配，不增加成功执行时每条指令的分配。

宿主 import 可以用 `CalxError::new_raw(message)` 报错。这类错误不假装拥有 VM 内部状态，`snapshot` 为 `None`。调用方可用 `stack()`、`top_frame()` 和 `globals()` 读取可选快照，不需要依赖 boxed 布局。

## 兼容性

#21 将原先位于 `CalxError` 顶层的 `stack/top_frame/globals` 移入公开的 `CalxErrorSnapshot`，属于实验期 Rust API 调整。`CalxError` 与 `CalxErrorSnapshot` 现从 crate root 导出；使用旧字段的 consumer 应改为 accessor 或 `error.snapshot`。

本地 `calcit-lang` 工作区审计没有发现直接访问这些旧字段的 consumer。未来发布前仍应在实际 Calcit binding 仓库执行编译验证。

## 大小与成功路径测量

调整前 Rust Clippy 报告 `CalxError` 至少为 144 bytes。调整后测试约束其 inline size 不超过 4 个 machine words；64-bit 目标上即不超过 32 bytes。VM snapshot 只在 trap 路径分配一次，`new_raw` 不分配 snapshot。

Criterion 使用独立 `main` checkout 作为 baseline，在同一 target 下对 50 samples、2 秒 warm-up、5 秒 measurement 复测：

- `instruction_execution`：变化置信区间 `-3.02%..+2.61%`，p=0.96；
- `multiple_calls`：变化置信区间 `-2.98%..+0.74%`，p=0.24。

两项均未检测到统计显著的成功路径性能变化。该调整的主要价值是 API/诊断布局和严格 lint，而不是宣称 VM 加速。

## 尚未承诺的部分

当前 message 仍是面向人的文本，不是稳定匹配接口。后续可以增加：

- `ParseError` 的统一结构；
- 稳定 diagnostic/trap code；
- Cirru source span；
- 精简调用栈摘要；
- 文本与 JSON renderer。

这些扩展应保留阶段边界，不把 `ValidationError` 和运行期 snapshot 再次合并。
