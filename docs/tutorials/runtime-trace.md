# 有界运行时 trace / Bounded runtime trace

## 中文

当 `check` 或 `explain` 已经通过、但运行结果或 trap 仍不符合预期时，使用：

```bash
calx trace demos/if.cirru
calx trace demos/if.cirru --function demo --limit 64
```

`trace` 执行真实的 Calx VM，不会重写或近似解释 guest 程序。每个事件固定显示 step、function、instruction
index、事件类别、lowered instruction、operand stack 前后状态、source span 与 frame depth。local/global 写入还会
显示对应 slot 的前后值。

默认上限为 10,000 个 VM transition。达到上限时，命令以 `CALX_TRACE_LIMIT` 失败并报告下一条将执行的
function、instruction 与 source span；这能安全检查意外循环，而不会产生无界终端输出。`--limit N` 可为一个
特定教学或调试场景收紧或提高该限制。

`--function NAME` 只过滤**输出**到指定 function，程序仍从 `main` 按正常语义运行。因此它不会改变 call、branch
或 host effect 的执行顺序。unknown function 会在运行前被拒绝。

普通 `calx run`、`CalxVM::run` 与 `CalxVM::run_typed` 不创建 trace event 或复制逐步 stack snapshot；只有显式
`calx trace` 或 `CalxVM::run_traced` 才承担该诊断开销。

## English

When `check` or `explain` succeeds but a result or trap remains unexpected, run:

```bash
calx trace demos/if.cirru
calx trace demos/if.cirru --function demo --limit 64
```

`trace` executes the real Calx VM; it does not rewrite or approximately interpret guest code. Each event
deterministically shows the step, function, instruction index, transition kind, lowered instruction, operand
stack before and after, source span, and frame depth. Local/global writes also show their slot values before
and after.

The default cap is 10,000 VM transitions. On exhaustion, the command fails with `CALX_TRACE_LIMIT` and reports
the next function, instruction, and source span. This makes accidental loops safe to inspect without unbounded
terminal output. Use `--limit N` to tighten or raise the cap for a particular teaching or debugging scenario.

`--function NAME` filters **printed output** to one function while the program still starts at `main` and runs
with normal semantics. It therefore does not change call, branch, or host-effect ordering. An unknown function
is rejected before execution.

Normal `calx run`, `CalxVM::run`, and `CalxVM::run_typed` do not construct trace events or copy per-step stack
snapshots. Only explicit `calx trace` or `CalxVM::run_traced` pays that diagnostic cost.
