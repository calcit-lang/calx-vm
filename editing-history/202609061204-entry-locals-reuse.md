# Reuse entry locals capacity / 复用 entry locals 容量

## English

- Evidence from `calcit-calx-bench` PR #14 shows one input-size-independent locals reallocation per warm `gather-sum` entry run, while the otherwise equivalent no-extra-local dot product has none.
- Reset an entry by taking, clearing, reserving, and repopulating the existing top-frame locals vector. Old values and buffers are released, while its widest capacity remains available across repeated runs.
- Preserve strict `Uninitialized` slots, checked size overflow, argument/result validation, traps, source diagnostics, and the existing public API/value/instruction domains.
- Add regressions for ordinary repeated resets and rerunning after a trap, including stale-slot clearing and allocation identity reuse.
- Track the paired accept/reject performance experiment in #66; the implementation is not accepted on code shape alone.

## 中文

- `calcit-calx-bench` PR #14 的证据显示：warm `gather-sum` 每次 entry run 有一次与输入规模无关的 locals reallocation，而没有额外 local 的等价顺序 dot-product 为零。
- entry reset 改为取出、清空、预留并重新填充已有 top-frame locals vector；旧值和 buffer 仍及时释放，同时最宽容量可跨重复运行保留。
- 保持 strict `Uninitialized` slot、容量溢出检查、参数/返回值校验、trap、source diagnostic 以及现有公共 API/值域/指令域不变。
- 新增普通重复 reset 与 trap 后再次运行的回归，验证陈旧 slot 清理和 allocation identity 复用。
- 修改必须通过 #66 的配对接受/拒绝性能实验；不能只凭代码形态认定优化成立。
