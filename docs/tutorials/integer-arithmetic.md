# 1. 整数算术 / Integer arithmetic

## 中文

打开 [arithmetic.cirru](programs/arithmetic.cirru)。`fn main (-> i64)` 声明无参数、一个 I64 结果；`local $answer i64` 显式声明 storage 类型，使 CLI 选择 strict typed module。local 必须先赋值再读取，不以 Nil 填空。

`i.add (const 20) (const 22)` 是 folded Cirru：先压入两个值，再做加法。右端为栈顶，类型栈从 `[]` 到 `[I64, I64]` 再到 `[I64]`。编辑这两个整数即可编写自己的例子。

## English

Open [arithmetic.cirru](programs/arithmetic.cirru). The signature takes no arguments and returns I64. The typed local selects the strict module path and must be assigned before reading; Nil is not an initialization sentinel. Folded operands are expanded before the operation. Change the two integer constants to write your own program.

Calx I64 is not Calcit Number: the current Calcit producer uses F64 for Number. Do not infer that arbitrary Calcit numeric code can use this integer tutorial unchanged. See the bounded Wasm mapping below.

## 命令与输出片段 / Commands and output excerpts

<!-- cli-test: success -->

```bash
cargo run --quiet -- check docs/tutorials/programs/arithmetic.cirru
```

```text
[calx check] ok: 1 function(s), 6 syntax instruction(s), strict typed
```

<!-- cli-test: success -->

```bash
cargo run --quiet -- run docs/tutorials/programs/arithmetic.cirru
```

```text
Value(I64(42))
```

## 错误 / Error

[type-error.cirru](programs/type-error.cirru) 将 22 改为 22.，后者是 F64；不会隐式转换。
The F64 operand is rejected during validation, before execution. The location points to the addition.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- check docs/tutorials/programs/type-error.cirru
```

```text
error[CALX_VALIDATION] validation
type-error.cirru:3:3
expected I64, found F64
```

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)

