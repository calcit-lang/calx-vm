# Bounded runtime trace / 有界运行时 trace

## 中文

新增 `VmObserver`、owned `VmEvent` 和 `CalxVM::run_traced`。事件来自实际 `step()` 路径：每条记录保留
function、instruction index、lowered instruction、source span、frame depth、stack 前后状态，并为 call、return、
branch、local/global 写入和 trap 标记 transition kind。普通 `run` / `run_typed` 继续走 observer 为 `None` 的路径，
不构造事件或逐步复制 stack。

`calx trace FILE [--limit N] [--function NAME]` 为该 API 提供确定性文本入口。默认限制 10,000 个 transition；
超限使用 `CALX_TRACE_LIMIT` 说明下一条 instruction，避免无限循环无界输出。function 选项仅过滤显示，不改变从
main 开始的真实执行、call 顺序或 guest 语义。

测试覆盖 call、taken/not-taken branch、implicit return、source location、local/global slot transition、trap、
limit、function filter 和既有 legacy/strict runtime 路径。JSON renderer、TUI 与异步 runtime 仍不在范围内。

## English

Adds `VmObserver`, owned `VmEvent`, and `CalxVM::run_traced`. Events come from the real `step()` path: each
record retains the function, instruction index, lowered instruction, source span, frame depth, and stack before
and after, with transition kinds for calls, returns, branches, local/global writes, and traps. Normal `run` and
`run_typed` continue through an observer-free path and construct neither events nor per-step stack copies.

`calx trace FILE [--limit N] [--function NAME]` is the deterministic text entry point for that API. Its default
cap is 10,000 transitions; exhaustion uses `CALX_TRACE_LIMIT` to identify the next instruction and avoid
unbounded loop output. The function option filters display only: execution still starts at main with the real call
order and guest semantics.

Coverage fixes calls, taken/not-taken branches, implicit returns, source locations, local/global slot transitions,
traps, limits, function filtering, and existing legacy/strict runtime paths. A JSON renderer, TUI, and async
runtime remain out of scope.
