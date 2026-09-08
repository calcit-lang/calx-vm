# 3. Loop 与 branch / Loops and branches

## 中文

[loop.cirru](programs/loop.cirru) 从 0 开始，每圈加 1。`block (i64 -> i64)` 接收计数并在退出时返回计数；内层 `loop (i64 -> i64)` 的 label 接收下一圈参数。`dup` 保留计数，比较消耗副本并产生 Bool。`br-if 1` 退出外层 block，未满足条件则 `br 0` 回到内层 loop。深度 0 总是最内层 label，不是绝对指令地址。

## English

The block's label carries its result; the loop's label carries its next-iteration parameter. A duplicated counter feeds the comparison, leaving the original for either branch. Depth 1 exits the surrounding block; depth 0 continues the loop. Structured label intent is preserved when lowering to instruction jumps; see Wasm mapping for the shared subset and limits.

## 执行 / Execute

<!-- cli-test: success -->

```bash
cargo run --quiet -- run docs/tutorials/programs/loop.cirru
```

```text
Value(I64(3))
```

## 错误与限制 / Failure and limits

[loop-label-error.cirru](programs/loop-label-error.cirru) 使用不存在的深度 3：此处只有 loop、block、function 三个 label（深度 0、1、2）。
The invalid depth is rejected during validation; repair the target label before running.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- check docs/tutorials/programs/loop-label-error.cirru
```

```text
error[CALX_VALIDATION] validation
loop-label-error.cirru:6:7
invalid branch depth 3
```

将 trace budget 设得太小会失败，不是程序类型错误。它模拟调试循环时必须处理的有界执行中断；不能将部分 trace 当成成功结果。对真正的意外无限循环同样有效。
An intentionally insufficient trace budget fails before completion. This is a diagnostic budget error, not a type error or a successful partial result. Raise the budget after checking the branch logic.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- trace docs/tutorials/programs/loop.cirru --limit 3
```

```text
error[CALX_TRACE_LIMIT] runtime
trace step limit 3 exhausted
```

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)
