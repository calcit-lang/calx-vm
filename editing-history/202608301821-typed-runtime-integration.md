# Strict typed runtime integration / 严格类型运行时集成

## 中文

- 用 `ValidatedProgram` 串联 strict declaration validation、typed function validation 与 lowering，安全公开 API 无法把未验证 program 交给 strict VM。
- validator context 读取 declared locals、globals、global mutability 与 typed import signature；const global write 和 import mismatch 在执行前失败。
- strict lowering 将 import name 解析为稳定 declaration index，runtime 分离 zero-result 与 single-result callback，并检查宿主参数和返回值。
- local/global storage 使用 `CalxSlot::Uninitialized` 表示未初始化控制状态；strict 与 legacy read-before-set 都 trap，不再用 `Calx::Nil` 占位。
- strict completion 使用 `CalxRunResult::Void | Value`；legacy `run()` 仅在兼容 adapter 中把 void 映射回显式 Nil。
- CLI typed module 已接入 strict run/check/explain，同时保留独立 legacy 路径。
- 将 Calx VM executable state 改为私有并提供只读函数 accessor，避免构造后绕过验证修改 strict instructions。

## English

- Connect strict declaration validation, typed function validation, and lowering through `ValidatedProgram`, preventing safe public APIs from sending unvalidated programs into the strict VM.
- Build validator context from declared locals, globals, global mutability, and typed import signatures so const-global writes and import mismatches fail before execution.
- Lower typed import names to stable declaration indexes, separate zero-result and single-result callbacks, and check host arguments and results at the boundary.
- Represent uninitialized local/global control state with `CalxSlot::Uninitialized`; strict and legacy read-before-set now trap without storing `Calx::Nil` placeholders.
- Represent strict completion with `CalxRunResult::Void | Value`; only the legacy `run()` adapter maps void back to explicit Nil.
- Route typed CLI modules through strict run/check/explain while preserving a separate legacy path.
- Make executable Calx VM state private with a read-only function accessor so callers cannot mutate strict instructions after validation.
