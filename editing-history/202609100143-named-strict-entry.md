# Execute named strict entries / 执行严格具名入口

## English

- Allow a validated strict program to instantiate without a synthetic `main` function.
- Add typed and traced named-entry APIs that resolve only validated functions, check the selected signature, and share one reset/interpreter path.
- Keep `run_typed` and `run_traced` as `main` compatibility wrappers; missing names fail without fallback.
- Cover independent entries, void/value results, argument mismatches, recursion, import traps, repeated execution, trace limits, and source-aware runtime state.

## 中文

- 允许已经验证的 strict program 在不合成 `main` 函数时实例化。
- 增加 typed 与 traced 具名入口 API；只解析 validated functions，校验所选函数签名，并复用同一条 reset/interpreter 路径。
- 保持 `run_typed` 与 `run_traced` 作为 `main` 兼容 wrapper；缺失名称直接失败，不做回退。
- 覆盖独立入口、void/value 结果、参数不匹配、递归、import trap、重复执行、trace limit 与保留源码信息的运行状态。
