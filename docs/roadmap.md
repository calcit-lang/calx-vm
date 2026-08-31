# Calx VM 强化路线图

> 状态：草案  
> 面向版本：0.3—0.5  
> 项目定位：用于学习 WebAssembly 语义、验证栈式虚拟机设计，并探索把 Calcit 的计算密集型子集编译到较快执行层。

## 1. 定位与边界

Calx 不是 WebAssembly 的兼容实现，也不以生产部署为目标。它应当保留 Cirru/Calcit 风格的可读语法，同时为 Calcit 的计算密集型 typed subset 提供比通用动态运行时更严格的执行层：strict program 中不允许 Dynamic boundary，也不以 `nil` 充当未初始化、缺失或 void 哨兵。旧动态数据行为只留在显式 legacy profile。Calx 同时借用 WebAssembly 最有教学价值的设计：类型化操作数栈、结构化控制流、验证与执行分离、显式 trap、函数/宿主边界，以及可选的线性内存。

强化工作的优先级为：

1. 语义正确且不会因普通输入崩溃；
2. 能解释程序为什么有效、为什么失败、每步怎样改变 VM；
3. 能承接一个范围清楚的 Calcit 编译子集；
4. 在代表性计算上验证性能方向；
5. 最后才做有数据支持的局部优化。

明确不追求：

- 完整覆盖 WebAssembly 3.0、提案阶段特性或二进制兼容；
- JIT、复杂寄存器分配、平台专用汇编和极端内存布局优化；
- 线程、SIMD、GC、异常等大特性的全面实现；
- 用 benchmark 数字替代语义、测试和教学可读性。

## 2. 当前基线审计

仓库已经具备可继续演进的骨架：

- Cirru 折叠写法展开为栈式指令；
- `block`、`loop`、`if`、`br`、`br-if` 的结构化语法及跳转降级；
- 函数调用、尾调用、局部变量、全局变量、宿主 import；
- 整数/浮点基础运算、断言、栈和帧检查输出；
- demos、Criterion benchmark 和发布流程。

当前最值得先处理的风险是：

- `preprocess` 只跟踪栈高度，不跟踪栈中值类型；分支、调用、local/global 的类型一致性尚未形成真正的验证器；
- list/link/布尔指令已有 IR 占位但语义未定义；当前 parser 明确拒绝，interpreter 也返回错误，待 RFC 后再开放；
- 普通程序错误仍可能触发 `unwrap`、`todo!`、`unreachable!`、整数除零、非法 shift 或宿主进程退出，而不是返回可检查的 trap；
- 已看到 `local.tee` 被解析成 `LocalSet`、`global.set` 边界判断反向等明显语义缺口；
- `Calx::truthy` 与跳转/断言各自实现真假判断，规则不一致；
- `return`、不可达代码、循环结果、多返回值和分支目标的栈规则还不完整；
- CLI 暴露的 binary emit/eval 选项尚未实现，格式也没有 magic/version/兼容策略；
- 自动测试目前只有 1 个 parser test，demos 主要验证“能运行”，缺少结果断言、负向验证和 trap 测试；
- `CalxError` 的大型 inline `Err` payload 曾阻塞严格 Clippy；#21 已改为按需 boxed snapshot，后续需防止该 API 再次膨胀。

当前开发分支已完成第一批 M0 止血：修正 `local.tee`、`global.set`、`if` 分支栈恢复和嵌套 label 查找；整数 wrapping/trap 行为已固定；guest `unreachable`/`quit` 不再 panic 或终止宿主；未定义指令在 parser 入口明确拒绝。具体契约见 [`instruction-set.md`](instruction-set.md)。

M1 第一阶段已实现：独立 typed operand/control stack validator 在 lowering 前运行，已知类型错误提前返回 `ValidationError`；现有无类型 local/global/import 被显式识别为 legacy `Dynamic` 边界。设计见 [`RFC 0001`](../RFCs/0001-validation-and-traps.md)。

M1 第二阶段的 typed local/global/import module contract 已由 [`RFC 0002`](../RFCs/0002-typed-boundaries.md) 固定并完成。`ValidatedProgram` 串联 declaration、typed validator 与 lowering；strict VM 使用 declared locals/globals、indexed imports、typed host binding 和 `CalxRunResult`，并以独立 Uninitialized slot state 取代 nil 哨兵。CLI 的 typed module 进入 strict run/check/explain，旧动态 API 留在单独 legacy profile 渐进迁移。

M2 第一阶段已实现：`calx check` 可只解析和验证，`calx explain` 可观察 folded Cirru、展开 syntax、逐指令类型/控制栈变化和 lowering 结果。用法见 [`tutorials/check-and-explain.md`](tutorials/check-and-explain.md)。

M0 错误布局债务 #21 已完成：`CalxError` 缩小为 message 与可选 boxed snapshot，宿主错误不再携带伪 VM 状态，严格 Clippy 可作为常规门禁。错误阶段与兼容性见 [`diagnostics.md`](diagnostics.md)。

M0 收口审计 #29 已完成：逐 opcode 的 parser/validator/lowering/interpreter/test 状态由 [`instruction-matrix.md`](instruction-matrix.md) 跟踪；malformed structured forms 和手工构造的非法 public IR 返回错误而不是 panic；全部 demos 由 `try.sh` 断言稳定结果；CI 与 publish workflow 显式执行严格 Clippy 和 crates.io-style package verification。

## 3. 目标架构

建议明确四层，并让教学工具可以观察每层：

```text
Cirru source
  -> parser + source spans
CalxSyntax（结构化、保留名字）
  -> validator + lowering
ValidatedProgram / CalxInstr（索引化、带签名）
  -> interpreter
VM events + result/trap
```

- parser 只负责语法与源位置，不偷偷决定运行时语义；
- validator 使用“操作数类型栈 + 控制栈 + 函数/局部/global/import 上下文”；
- lowering 只在验证通过后生成跳转和索引；
- interpreter 假定输入已验证，但仍把动态边界错误统一报告为 trap；
- `check`、`explain`、`trace` 复用同一套中间结果和事件，不另写一套近似逻辑。

## 4. 分阶段计划

### M0：0.2.x 语义止血与测试地基

目标：任何能从 CLI 或公开 API 到达的普通错误都返回诊断，不 panic；现有行为先被测试固定。

工作项：

- 给每条已暴露指令建立状态矩阵：`parsed / validated / lowered / executed / tested / documented`；
- 修正 `local.tee`、`global.set`、真假判断、栈下溢、除零、shift、算术溢出等已知问题；
- 将 `Unreachable` 定义为 trap，将 `Quit` 限制在 CLI 边界，库执行不得直接 `process::exit`；
- 未实现指令在 parser/validator 阶段明确拒绝，或者完整实现后再开放；
- 已把 `CalxError` 改为轻量 message 与按需 VM snapshot；稳定错误种类、源位置和调用栈摘要继续按 diagnostic RFC 演进；
- 将 demos 变成可断言的集成测试，添加每条指令的成功、类型错误、越界和 trap 用例；
- CI 固定执行 `cargo fmt --check`、`cargo test`、demos/集成测试和 Clippy；nightly 仅在确有教学或诊断价值时保留；
- 保留 crates.io 无 lockfile 构建检查，避免 parser 宽松 semver 再次破坏已发布 crate。

验收：

- 公开输入路径中没有 `todo!()`、因用户程序触发的 `unwrap`/`unreachable!` 和进程级退出；
- 每个可解析 opcode 至少有一条执行测试或一条“明确不支持”的验证测试；
- Debug/Release 对算术和 trap 的可观察结果一致；
- CI 的强制检查与 `AGENTS.md` 一致且全部可复现。

### M1：0.3 类型验证器与统一 trap

目标：把目前的“栈高度预处理”升级为教学友好的单遍类型验证器。

工作项：

- 引入 `ValidationContext`、typed operand stack、control frame 和 `ValidationError`；
- 为每条指令定义输入/输出类型，不再只返回 `(usize, usize)`；
- 校验 local/global 索引及类型、函数/import 签名、返回值、block/loop/if 参数和结果；
- strict module 强制 zero Dynamic、禁止 nil-typed storage/function/import boundary；
- 用 void result 与独立 uninitialized state 取代隐式 `Calx::Nil` 哨兵；
- 按 WebAssembly 的思路实现不可达状态/栈多态，避免 `return`、`br` 后的死代码产生伪错误；
- 明确 Calx 与 Wasm 的差异：`i64` 条件、`bool`、`nil`、`str`、`list`、truthiness、动态值和尾调用；
- 将源 span 从 parser 贯穿到 syntax、lowered instruction 和错误报告；
- 形成中文 RFC：语义核心、验证算法、trap 分类、与 Wasm 的逐项对照。

验收：

- 验证通过的程序不会因操作数类型、local/global 类型或控制栈错误在解释器中崩溃；
- strict program 的 local/global 热路径无需 Dynamic 类型守卫，void/未初始化状态不产生 nil；
- 每类验证规则同时有正向和负向测试；
- 错误至少包含函数名、源位置、指令、期望栈和实际栈；
- RFC 中每项声称“对齐 Wasm”的行为都链接到相应规范规则或测试来源。

### M2：0.4 教学与检测工具

目标：不仅能运行 Calx，还能用它理解栈式 VM 和 Wasm 的验证/执行过程。

CLI 建议：

- `calx check FILE`：只解析和验证，不执行；
- `calx explain FILE [--function NAME]`：展示 folded Cirru、展开后的 `CalxSyntax`、lowered instruction，以及逐指令栈类型/控制栈变化；
- `calx trace FILE [--limit N]`：展示运行前后值栈、帧、local/global 变化、跳转和 trap；默认限制步数，避免死循环刷屏；
- `calx inspect FILE --format json`：输出稳定但明确标为 experimental 的机器可读报告，便于编辑器和教学页面使用；
- `calx compare CASE`：对支持的 Wasm 交集运行配对测试，输出相同点、预期差异和意外差异。

检测方法分三层：

1. **语义表驱动测试**：从规范规则人工提炼小而清晰的 Calx/WAT 配对案例；
2. **官方 testsuite 子集映射**：只映射 Calx 已声明支持的数值、变量、函数和控制流用例，记录不能映射的原因；
3. **有界差分/性质测试**：生成短小的有效程序，比较结果或 trap 类别；限制深度、步数和数值范围，保持 CI 快速可读。

不把某个重量级 Wasm runtime 放入核心库依赖。差分执行器应位于 dev-tool/可选 feature，CI 可使用固定版本的参考解释器或 Wasm CLI。

验收：

- 一条示例命令能完整解释“源码 → 验证 → 降级 → 执行”；
- trace 输出可复现、可限制，并能清楚表示 call、return、branch 和 trap；
- 差分报告区分“Calx 有意不同”“Calx 尚未实现”“实现疑似错误”；
- 文档至少包含算术、if、loop、函数调用、trap 五篇短教程。

### M3：0.5 Calcit 编译实验闭环

目标：选择一个现实但有限的 Calcit 子集，形成可测量的端到端编译实验。

首批子集建议：

- Calcit `Number -> F64`、`Bool` 与函数级 `Unit -> void`，局部绑定与赋值；strict
  boundary 不接受 Nil 或 Dynamic，也不根据数值字面量猜测 I64；
- 直接函数调用、尾调用、`if`、有界 loop；
- 少量显式宿主 imports；
- 首版限定 scalar kernel；后续按独立 RFC 增加同质 typed buffer，暂不复制 Calcit 全部持久化
  集合语义；严格 `F64Buffer` 的类型、ownership、index/trap 与 ABI edition 见
  [`RFC 0004`](../RFCs/0004-f64-buffer-abi.md)；
- 通过 wrapper 明确 Calcit 动态值与 Calx 类型值的转换成本。

工作项：

- RFC 定义 Calcit → Calx 的可编译语法、类型假设、失败/回退策略和宿主 ABI；
- 提供 Rust `ProgramBuilder`/validated IR API，编译器不必伪造 Cirru 文本；
- 建立 golden fixtures：Calcit 源、Calx IR、解释结果、错误和基准输入；
- 选择 `sum`、fibonacci、数值变换、小数组聚合等 5—8 个 kernel，同时比较 Calcit runner、Calx debug/release；
- benchmark 报告编译/转换成本与纯执行成本，防止只测热循环而误判端到端收益；
- 达不到收益的特性允许回退到 Calcit，不强求扩大 Calx 语义。

具体子集、closed-call-graph eligibility、all-or-nothing fallback、宿主 ABI 与计量边界见
[`RFC 0003`](../RFCs/0003-calcit-subset-and-host-abi.md)。Calcit Number 的首批条件分支依赖
#45 补齐 F64 comparison instructions；不得通过 I64 推断、truthiness 或动态 host import 绕过。
#37 已建立 source-aware `ProgramBuilder -> CalxProgram -> ValidatedProgram` 路径；后续 translator
应直接消费 typed preprocessed snapshot，并复用 opaque local/global/import handle 与结构化控制流 API。

验收：

- 至少 3 个来自真实 Calcit 写法的 kernel 可自动编译、验证和执行；
- Calcit 与 Calx 在固定输入 corpus 上结果一致；
- 性能文档能解释收益来自哪里、边界转换花费多少、哪些程序不适合编译；
- 编译子集和回退行为有用户可读的中文说明。

### M4：后续可选教学实验

这些方向覆盖面广，但必须逐项先写 RFC，不作为 0.3—0.5 的阻塞项：

- 最小线性内存：64 KiB page、load/store、越界 trap、`memory.size/grow`；
- `select`、`br_table`、只读 table 和 `call_indirect`；
- 版本化 binary container：magic、edition、section、长度校验和兼容测试；
- 可视化控制流图、栈高度图和 source-to-instruction 映射；
- 与浏览器 Wasm 或 Wasmtime 的教学对照 harness。

暂缓 SIMD、线程、GC、异常和 JIT；只有当它们能回答清晰的教学问题或解除 Calcit 编译实验的具体阻塞时再立项。

## 5. 建议 issue 切分

issue 与 PR 标题、正文统一使用中英双语。文章和 RFC 可以只用中文，但关键术语首次出现时保留英文。

按依赖顺序建议创建以下 issue；一个 issue 尽量对应 1—3 个可独立 review 的 PR：

1. `docs: 明确 Calx 定位及与 Wasm 的语义边界 / define Calx scope and Wasm semantic mapping`
2. `test: 建立指令语义矩阵与负向测试 / build instruction semantic matrix and negative tests`
3. `fix: 消除用户程序可触发的 panic / eliminate panics reachable from guest programs`
4. `fix: 修正 local/global、真假值与算术 trap / fix local/global, truthiness, and arithmetic traps`
5. `refactor: 分离解析、验证与降级 / separate parsing, validation, and lowering`
6. `feat: 实现类型化操作数栈验证器 / implement typed operand-stack validator`
7. `feat: 实现控制栈与不可达代码验证 / validate control stack and unreachable code`
8. `feat: 增加 check、explain 与 trace 命令 / add check, explain, and trace commands`
9. `test: 建立 Wasm 语义子集差分验证 / add differential checks for the Wasm semantic subset`
10. `rfc: 定义 Calcit 到 Calx 的编译子集与宿主 ABI / define the Calcit-to-Calx subset and host ABI`
11. `feat: 提供 validated IR 与 ProgramBuilder API / provide validated IR and ProgramBuilder APIs`
12. `test: 建立 Calcit 端到端 fixtures 与基准 / add end-to-end Calcit fixtures and benchmarks`
13. `rfc: 设计最小线性内存实验 / design a minimal linear-memory experiment`
14. `rfc: 设计版本化 Calx binary container / design a versioned Calx binary container`

`#21 perf: 评估 CalxError 大返回类型 / evaluate large CalxError returns` 已在 M0 完成：大型 VM 状态移入可选 boxed snapshot，严格 Clippy 恢复为门禁；该改动没有夹带不相关的 VM 微优化。

## 6. 双语 issue 与 PR 规范

### Issue 模板

```markdown
## 中文

### 背景
### 目标
### 非目标
### 验收标准

## English

### Context
### Goals
### Non-goals
### Acceptance criteria
```

### PR 模板

```markdown
## 中文

- 变更：
- 原因：
- 验证：
- 兼容性/已知限制：

## English

- Changes:
- Rationale:
- Verification:
- Compatibility / known limitations:
```

标题建议使用 `type: 中文 / English`。两种语言表达相同事实即可，不要求逐字翻译；命令、错误码、API 名和 benchmark 数据只保留一份，避免双语正文漂移。

## 7. RFC 与文档布局

建议逐步形成：

```text
docs/
  roadmap.md
  instruction-set.md
  tutorials/
  wasm-mapping.md
  diagnostics.md
RFCs/
  0001-validation-and-traps.md
  0002-typed-boundaries.md
  0003-calcit-subset-and-host-abi.md
  0004-f64-buffer-abi.md
```

linear memory 与 binary container 仍是未立项的 M4 候选；在对应 issue/RFC 实际创建前不预占文件名
或编号，避免路线图把计划误写成现有文档。

RFC 使用中文维护，并固定包含：动机、术语、具体语义、与 Wasm 的相同/不同点、替代方案、测试计划、教学示例、兼容性和未决问题。

## 8. 性能原则

- 先建立正确性 corpus，再接受优化；
- 每个优化 PR 必须说明假设、基准场景、噪声控制和回归测试；
- 关注可解释的热点：指令 dispatch、frame 切换、值 clone、宿主边界和动态类型检查；
- 不以微基准中的单点提升换取 unsafe 扩散或语义分叉；
- unsafe 只在 profile 证明必要、边界可局部证明且有回归测试时保留；
- 同时报告 debug、release 和端到端 Calcit 场景，不追逐脱离目标 workload 的极限数字。

## 9. 推荐的第一批 PR

第一批不直接重写 VM，按以下顺序降低风险：

1. **PR A：语义矩阵 + 回归测试**——固定现状并暴露 `local.tee`、`global.set`、trap 等失败案例；
2. **PR B：panic → trap**——消除 guest 可触发 panic，统一错误分类，不改变控制流架构；
3. **PR C：验证器 RFC + typed stack 骨架**——先定义规则，再替换 `stack_arity`；
4. **PR D：结构化控制验证**——完成 block/loop/if/br/return 的控制栈规则；
5. **PR E：`calx check` 与 `calx explain`**——让新验证器立即产生教学价值。

完成 PR A/B 后再决定未实现的 list/bool/link 指令是实现还是暂时从语法入口撤下；完成 M1 后再进入线性内存和 Calcit 编译闭环。

## 10. 参考基线

- WebAssembly Core Specification：<https://webassembly.github.io/spec/core/>
- Validation Algorithm：<https://webassembly.github.io/spec/core/appendix/algorithm.html>
- WebAssembly spec、reference interpreter 与官方 tests：<https://github.com/WebAssembly/spec>
- 汇总 testsuite mirror：<https://github.com/WebAssembly/testsuite>

Calx 的目标不是让上述测试全部通过，而是对每个声称支持的交集，能够指出对应规则、测试证据和有意差异。
