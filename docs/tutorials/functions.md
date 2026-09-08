# 4. 函数与尾调用 / Functions and tail calls

## 中文

[calls.cirru](programs/calls.cirru) 的 main 普通调用 countdown，参数从 3 递减到 0 后返回 42。`call` 保留调用者 continuation；`return-call` 将当前函数的返回责任交给被调用函数，不增加递归 frame 深度。签名规定参数与结果，不接受 Dynamic 代替证明。

## English

A normal call retains the caller's continuation. The tail call transfers the current return obligation to the target; repeated countdown calls keep the same frame depth. Arguments and results obey the fixed signature. Calcit lowering can use this operation for eligible tail-position recur; this source example is a VM tutorial, not evidence of application-level speedup.

## 执行与观察 / Execute and observe

<!-- cli-test: success -->

```bash
cargo run --quiet -- run docs/tutorials/programs/calls.cirru
```

```text
Value(I64(42))
```

<!-- cli-test: success -->

```bash
cargo run --quiet -- trace docs/tutorials/programs/calls.cirru --limit 100
```

```text
call countdown Call(1)
tail-call countdown ReturnCall(1)
frames 1 -> 1
[calx trace] result: Value(I64(42))
```

## 错误 / Error

[call-error.cirru](programs/call-error.cirru) 将 F64 传给 I64 参数；错误属于 call site 的 validation，而不是函数运行到一半的 trap。
The wrong argument type is rejected at the call site before running the callee.

<!-- cli-test: failure -->

```bash
cargo run --quiet -- check docs/tutorials/programs/call-error.cirru
```

```text
error[CALX_VALIDATION] validation
call-error.cirru:3:3
expected I64, found F64
```

[目录 / Contents](README.md) · [诊断 / Diagnostics](../diagnostics.md) · [Trace](runtime-trace.md) · [Wasm mapping](../wasm-mapping.md)

