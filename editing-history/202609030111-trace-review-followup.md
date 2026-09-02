# Runtime trace review follow-up / 运行时 trace review 跟进

## 中文

处理 #58 review 后，offset branch 事件改为始终从执行前 instruction index 与 offset 推导目标地址，因而
`JmpOffsetIf` 不跳转时也不会把当前指令误报为目标。trap event 不再附带 local/global slot change；写槽位
仅代表已成功完成的写入 transition。

每次执行现在从新的 main frame 重置。这样 trace 因 limit 停在 callee，或 callee 内 trap 后，复用同一 VM
仍会从 main 开始下一次执行；global 的既有跨运行存续语义不变。补充了这三类回归测试。

## English

After addressing #58 review, offset-branch events always derive their destination from the pre-step instruction
index and offset, so a non-taken `JmpOffsetIf` no longer reports its current instruction as the target. Trap
events no longer carry local/global slot changes: a slot change now represents only a completed write transition.

Each execution now rebuilds a fresh main frame. A VM reused after a trace limit or a callee trap therefore starts
the next execution at main; the existing cross-run persistence of globals is unchanged. Regression tests cover
all three cases.
