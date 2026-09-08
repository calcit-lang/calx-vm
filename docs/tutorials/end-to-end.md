# 6. 从源码到执行事件 / From source to runtime events

## 中文

沿用 [arithmetic.cirru](programs/arithmetic.cirru)，不需要新 API。

1. folded Cirru 的 `i.add (const 20) (const 22)` 表达源码嵌套。
2. parser 将其展开为两个 Const 和一个 CalxSyntax::IntAdd，保留源位置。
3. typed validator 证明 `[I64, I64] -> [I64]`，并检查 local 的赋值与返回契约。
4. lowering 产生 VM 实际执行的 IntAdd；这里索引 2 与源码第 3 行对应。
5. trace 在真实解释器执行时记录值栈 `[I64(20), I64(22)] -> [I64(42)]`，不是另一套模拟器。

## English

Follow the same arithmetic source through folded syntax, expanded CalxSyntax, typed stack validation, lowered instructions and actual runtime transitions. `check` validates without guest execution; `explain` adds lowering without execution; `run` executes; `trace` executes with bounded observation. Only explicit tracing pays per-event snapshot cost.

## 验证、lowering 与执行 / Validation, lowering and execution

<!-- cli-test: success -->

```bash
cargo run --quiet -- check docs/tutorials/programs/arithmetic.cirru
```

```text
[calx check] ok: 1 function(s), 6 syntax instruction(s), strict typed
```

<!-- cli-test: success -->

```bash
cargo run --quiet -- explain docs/tutorials/programs/arithmetic.cirru --function main
```

```text
folded Cirru:
(i.add (const 20) (const 22))
syntax[002] IntAdd
source: docs/tutorials/programs/arithmetic.cirru:3:3
operand: [I64, I64] -> [I64]
lowered: IntAdd
```

<!-- cli-test: success -->

```bash
cargo run --quiet -- trace docs/tutorials/programs/arithmetic.cirru --limit 16
```

```text
main@2 instruction IntAdd stack [I64(20), I64(22)] -> [I64(42)]
[calx trace] result: Value(I64(42))
```

## 错误 / Error

用 F64 操作数破坏类型契约时，explain 在 validation 阶段失败，不会执行 IntAdd。
With an F64 operand, explain fails validation instead of running or silently coercing the value.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- explain docs/tutorials/programs/type-error.cirru
```

```text
error[CALX_VALIDATION] validation
expected I64, found F64
```

## 边界 / Boundaries

Calcit 负责 typed snapshot、eligibility、cache 与应用生命周期；VM 负责验证、lowering、执行和 trap。这里的文件是 Calx 源码，不是 Calcit 程序，也不是 Wasm 文本。不要把这条 CLI 路径当成 Calcit 全程序编译模式。

Calcit owns source eligibility, snapshots, caching and application lifecycle. The VM owns validation, lowering, execution and traps. This is Calx source, not Calcit source or Wasm text; the CLI walkthrough does not introduce a whole-program Calcit backend mode.

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)
