# Preserve entry locals after nested traps / 嵌套 trap 后保留 entry locals

## English

- PR #67 review identified that an ordinary-call trap leaves the entry frame at the bottom of `frames` and the callee in `top_frame`.
- Entry reset now takes reusable locals from the bottom entry frame when a call stack remains, while preserving the existing top-frame path for empty-stack and tail-call execution.
- Add a `main -> helper -> trap` regression that runs twice and verifies the actual entry locals pointer/capacity are reused and stale values are replaced.
- Runtime errors, source diagnostics, frame order, strict slots, and public contracts remain unchanged.

## 中文

- PR #67 review 指出：普通调用发生 trap 时，entry frame 位于 `frames` 底部，callee 才是 `top_frame`。
- entry reset 现在会在遗留调用栈非空时从底部 entry frame 取出可复用 locals；空调用栈和尾调执行仍沿用 top-frame 路径。
- 新增运行两次的 `main -> helper -> trap` 回归，验证真正 entry locals 的指针/容量被复用，并且陈旧值已替换。
- runtime error、source diagnostic、frame 顺序、strict slot 与公共契约保持不变。
