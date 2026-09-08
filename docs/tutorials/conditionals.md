# 2. 条件与结果 / Conditions and results

## 中文

[conditional.cirru](programs/conditional.cirru) 用 `i.lt` 比较两个 I64，产生 Bool。两个 `do` 分支都必须满足 `if (-> i64)` 的结果契约。把 2 改成 4 会选择返回 0 的分支。strict 教程使用 Bool，不依赖 legacy numeric truthiness。

## English

The comparison produces Bool. Both branches must leave an I64 as declared by the if signature, regardless of which branch is selected at runtime. Change 2 to 4 to take the zero-valued branch. Calcit's strict producer also requires Bool conditions; Wasm's condition representation differs (see mapping).

## 执行 / Execute

<!-- cli-test: success -->

```bash
cargo run --quiet -- run docs/tutorials/programs/conditional.cirru
```

```text
Value(I64(42))
```

## 错误 / Error

[branch-error.cirru](programs/branch-error.cirru) 的 else 返回 F64，即使这次运行不会选中，也必须拒绝。
The invalid else result is checked even when the condition would select the other branch. This is a static contract, not speculative execution.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- check docs/tutorials/programs/branch-error.cirru
```

```text
error[CALX_VALIDATION] validation
expected I64, found F64
```

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)

