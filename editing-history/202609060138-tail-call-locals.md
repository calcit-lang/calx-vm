# Reuse tail-call locals / 复用尾调用局部存储

## 中文

- 关联 #60；独立 calcit-calx-bench#7 的 clean published-0.4.0 基线显示 range-sum/dot-product 分配次数随循环 size 线性增长。分配计数不代表耗时占比，性能对照与正式发布消费链继续单独追踪。
- ReturnCall 在完成 target/operand 检查后清空现有 locals，将 operand tail drain 到同一 Vec，重新补齐 strict Uninitialized slots，再更新 frame 元信息。
- 保留 caller stack prefix；容量只在布局需要时增长，保持到该 frame 被释放/reset，不新增 pool、公共 API、nil/dynamic、unsafe 或 opcode。
- 6 个集成测试覆盖自/相互尾调、不同 arity/local 布局、旧 operand 丢弃、buffer 转发/释放、未初始化 trap/span、trace 与重复 run/legacy local.new。
- 2 个内部测试验证 500 次宽窄布局切换时容量与地址稳定、operand underflow 前不修改 frame。容量测试已确认在旧实现上失败。
- 验证：fmt、debug/release 全量 tests、all-targets/all-features Clippy -D warnings、全部 try.sh demos、cargo package --offline --allow-dirty 通过。首轮 Cargo index 被 sandbox proxy 阻止，改用本地缓存 offline 完成。

## English

Implements the mechanism stage of #60 after the standalone clean published-VM
baseline in calcit-calx-bench#7 established linear tail-kernel allocation counts.
Counts are not a timing attribution; comparison and published downstream adoption
remain separate gates.

After target/operand guards, clear the existing locals, drain arguments into its
retained Vec, append strict uninitialized slots and replace frame metadata. Keep
the caller operand prefix and existing frame-release/reset lifecycle. No pool,
public API, dynamic value, unsafe access or opcode is introduced.

Six integration tests cover tail recursion, layout changes, discarded operands,
buffer forwarding/release, uninitialized traps/source spans, deterministic tracing,
repeated runs and legacy locals. Two internal tests establish stable capacity over
500 wide/narrow transitions and guard-before-mutation behavior; the capacity test
fails on the old implementation. Formatting, debug/release tests, warning-free
all-target/all-feature Clippy, every demo and package verification passed using
cached offline dependencies after the sandbox proxy rejected the initial index query.
