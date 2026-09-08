# 5. Trap 与源码诊断 / Traps and source diagnostics

## 中文

[trap.cirru](programs/trap.cirru) 的两个除法参数都是 I64，所以类型验证成功；除数为零则属于运行期 trap。类型安全不意味着每个输入都能正常返回。运行失败不会返回 Nil，也不能被当作成功；Calcit kernel runtime trap 后不能自动重跑 native。

## English

Both division operands are I64, so validation succeeds. Division by zero traps at runtime. A proven type is not proof that all inputs succeed. A trap is not a Nil result; the Calcit kernel integration must not retry native execution after a runtime failure. Parse, validation, runtime and host failures have distinct owners (linked below).

## 检查 / Check

<!-- cli-test: success -->

```bash
cargo run --quiet -- check docs/tutorials/programs/trap.cirru
```

```text
[calx check] ok: 1 function(s), 6 syntax instruction(s), strict typed
```

## 错误 / Error

trace 在 stdout 记录发生 trap 的 transition，stderr 给出稳定错误码、函数和源位置。不要把性能计时或整段 VM snapshot 当作稳定协议。修复时将文件中的除数改为非零 I64，再检查和执行。
Trace emits the trapping transition on stdout and the diagnostic on stderr. Repair the denominator, then check and run again. Timing and the full debug snapshot are not stable machine protocols.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- trace docs/tutorials/programs/trap.cirru --limit 16
```

```text
error[CALX_RUNTIME_TRAP] runtime
trap.cirru:3:3
in function main at syntax[2]
integer divide by zero
```

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)

