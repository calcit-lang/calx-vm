# Calx compilation roadmap / Calx 编译目标路线图

Tracking: [#61](https://github.com/calcit-lang/calx-vm/issues/61).
Status: experimental. Published baseline: 0.4.0. Next work uses the existing
0.5 compilation milestone; version numbers here do not promise a release date.

## 中文

Calx 的主线是承接 Calcit 静态计算子集，以可测量的端到端收益支持性能优化。
核心流程保持 `ProgramBuilder → CalxProgram → ValidatedProgram → CalxVM::run_typed`：
编译器消费 typed snapshot，验证器证明指令类型与控制流，VM 执行并保留源码 trap。
check/explain/trace 复用这些阶段，便于诊断和理解程序。

已经完成的基线：

- #22/#24/#29/#30：指令语义、安全输入、typed operand/control validator、source diagnostics；
- #31/#37/#45：strict local/global/import、显式 void/uninitialized、ProgramBuilder 与 F64 比较；
- #36/#38：Calcit closed-call-graph eligibility、scalar lowering、golden 与 differential corpus；
- #50–#53：独立 F64Buffer、checked index/bounds、ABI /2 和真实 dot-product 编译；
- #39：第一阶段 standalone 性能证据、公平 cached Calcit 基线、revision-safe artifact cache 与 copy-boundary 成本；
- #26/#32：check/explain 与有界 trace。

这些已验收阶段保持关闭。首轮报告证明有限 kernel 可获益，并不代表任意 Calcit 程序都适合 Calx。

接下来按依赖推进：

1. [#59](https://github.com/calcit-lang/calx-vm/issues/59)：封闭 strict 值域，拒绝没有元素证明的 List、
   Nil 常量与控制签名漏洞。规则见 [strict 值域与迁移](strict-value-domain.md)，此项是当前实现。
2. [#60](https://github.com/calcit-lang/calx-vm/issues/60)：先测 ReturnCall 的逐次 locals 分配；
   profile 支持时复用当前 frame 容量，验证不同调用布局、引用释放、trap 与 trace，再做相同源码前后对照。
3. 按真实 consumer 需求选择第二种 buffer 访存 workload；扩展 nominal/generic lowering 前先完成
   Calcit #842/#843/#797 的类型证明与 call contract。未知类型必须在 lowering 前失败，不能降成 Dynamic。

职责与边界：

- 本仓库拥有 VM 类型/验证/执行/source mapping 与 correctness；
- Calcit core 拥有 typed snapshot、eligibility/lowering、ABI 和 cache 语义；
- calcit-calx-bench 拥有机器采样、raw reports、环境/provenance、跨机器与端到端证据；
- 外部 Wiki 提供生态发现，具体契约以各仓库版本化文档/测试为准；
- calcit-calx 模块的生态升级/发布独立追踪，不等于 Calcit compiler target 的交付。

保留一条 validator/lowering/interpreter 主干。当前不增加 VM pool、自动 offload、JIT/SIMD、
通用 collection、完整新类型系统或调度层。每项优化都先回答实际成本在哪里、要删掉哪项成本、
怎样保留错误语义；bounds/conversion/host-result 守卫不可仅凭速度目标删除。
不把机器 crossover 固定成 correctness gate，也不在 VM trap 后自动重跑 Calcit。

后续任务 #33 JSON inspect、#34 Wasm mapping、#35 教程继续开放为非阻塞后续；已有教学工具继续维护。
本轮编译实验不加入 Calcit 0.13.78 release gate。版本发布必须有明确已发布依赖和对应验证，
实验 revision 仅作为复现证据，不冒充正式版本。

保留的延期候选（原 M4；尚未立项，不是取消）：

- 最小线性内存：64 KiB page、load/store、越界 trap、memory.size/grow；
- select、br_table、只读 table 和 call_indirect；
- 版本化 binary container：magic、edition、section、长度校验和兼容测试；
- 控制流图、栈高度图及 source-to-instruction 可视化；
- 浏览器 Wasm/Wasmtime 对照 harness。

每项须先说明具体 consumer 或教学问题、现有能力为何不足，再建立独立 issue/RFC；
不能仅因列在候选中就扩大当前版本。线性内存和 binary container 不预占 RFC 文件名或编号。

长期验收原则继续有效：

- 优化前先有正确性 corpus；每条公开 opcode 必须有执行或明确拒绝的测试。
- 每个优化 PR 说明假设、基准 workload、噪声控制与回归；同时看 debug、release、端到端成本。
- 热点候选包括 dispatch、frame 切换、值 clone、宿主边界和类型检查，#60 只是当前选择。
- 不为单点微基准扩大 unsafe 或分叉语义；引入 unsafe 必须有 profile 必要性、局部边界证明和回归。
- guest 普通错误返回诊断/trap；保持 source mapping、有限 trace 和可复现的公开输入检查。
- Wasm 交集须链接规范规则、测试证据及有意差异；差分工具留在开发工具层，不给核心引入重量级 runtime。

协作约定仍为中英文分区 Issue/PR，记录范围、依赖、验收与兼容性；RFC 保留动机、具体语义、
替代方案、测试计划和未决问题。完整旧阶段计划可查 Git 历史，当前已完成项以以上 issue 索引为入口。

## English

Calx targets static Calcit computational subsets with measurable end-to-end benefits.
Keep one ProgramBuilder → CalxProgram → ValidatedProgram → run_typed path. The compiler consumes
typed snapshots, validation proves instruction/control contracts, and execution preserves source-aware
traps. check/explain/trace reuse those stages.

Completed foundations include semantic/input-safety tests, typed validation and source diagnostics
(#22/#24/#29/#30), strict boundaries and builders (#31/#37/#45), scalar compilation (#36/#38),
the F64Buffer dot-product slice (#50–#53), initial standalone performance evidence and artifact caching
(#39), and check/explain/trace (#26/#32). Keep these accepted phases closed. Their finite kernel results
do not establish a universal Calcit speedup.

Issue #59's closed strict value domain is implemented in this change and remains unreleased.
Next, use #60 to profile per-tail-call allocation and,
if warranted, reuse frame-local capacity with layout/ownership/trap/trace tests and identical-source
before/after evidence. A later workload must have a named consumer. Nominal/generic lowering depends
on Calcit #842/#843/#797 proof/call contracts; unknown types fail before lowering instead of becoming Dynamic.

The VM owns execution mechanisms and correctness, Calcit owns compiler/ABI/cache semantics, and
calcit-calx-bench owns measurements and raw evidence. Ecosystem discovery lives in the external Wiki;
repository documents/tests remain the contract sources. The calcit-calx module release is separate.

No new VM pool, automatic offload, JIT/SIMD, general collection system, replacement type system, or
scheduler is planned. Preserve bounds/conversion/host-result guards and no Calcit retry after traps.
Never turn machine-specific crossover measurements into correctness gates. #33/#34/#35 remain open,
nonblocking follow-ups. This experimental work does not block Calcit 0.13.78.

Deferred candidates from the former M4 remain discoverable: minimal linear memory
(64 KiB pages, checked load/store, size/grow), select/br_table/read-only tables/call_indirect,
a versioned binary container, control-flow/stack/source visualizations, and a browser-Wasm/Wasmtime
comparison harness. These are not cancelled or approved implementation work. Each needs a concrete
consumer or teaching question and its own issue/RFC; do not reserve RFC filenames or numbers early.

Durable acceptance rules remain: establish correctness corpora before optimization; test execution
or explicit rejection for every public opcode; record hypotheses, workloads, noise control, regressions,
and debug/release/end-to-end costs. Consider dispatch, frames, cloning, boundaries, and type checks;
#60 is the current selection, not the entire optimization space. Unsafe requires profiling evidence,
local boundary proofs, and regression tests; a microbenchmark alone cannot justify semantic forks.
Preserve guest traps, source mapping, bounded traces, and reproducible input checks. Wasm intersection
claims need specification/test evidence and intentional-difference notes, with heavyweight reference
runtimes confined to development tooling. Keep separate Chinese/English collaboration records and
RFC motivation, semantics, alternatives, tests, and open questions. Git history retains the original
stage plans; the completed-issue index above is the current discovery entry.

## Specification references / 规范参考

- [WebAssembly Core Specification](https://webassembly.github.io/spec/core/)
- [Validation Algorithm](https://webassembly.github.io/spec/core/appendix/algorithm.html)
- [Specification, reference interpreter, and tests](https://github.com/WebAssembly/spec)
- [Testsuite mirror](https://github.com/WebAssembly/testsuite)
