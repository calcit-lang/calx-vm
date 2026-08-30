# RFC 0003：Calcit → Calx 编译子集、回退策略与宿主 ABI

- 状态：提议
- 目标版本：0.5 实验闭环
- 关联 Issue：#36
- 前置：RFC 0002 / #31 strict typed execution path
- 后续实现：#37 `ProgramBuilder`、#38 translator 与 golden fixtures、#39 性能评估

## 1. 摘要

Calx 承接 Calcit 中一个可静态证明、可整体回退、可独立计量的 typed scalar kernel 子集，
而不是成为第二个完整 Calcit runner。编译器位于 Calcit frontend/adapter 一侧，读取已经完成
宏展开、符号解析和类型预处理的 `CompiledProgram` snapshot，从显式入口提取封闭的 reachable
call graph，再通过 `ProgramBuilder -> CalxProgram -> ValidatedProgram` 进入 strict VM。

首版只开放 `Number -> F64`、`Bool -> Bool` 和函数级 `Unit -> void`。所有函数参数、结果与
local 都必须是非 nil 的具体类型；`Dynamic` 数量必须为零。`Nil`、Optional/Option/Result、
字符串与集合、closure/HOF、rest/optional 参数、global/ref 和任意动态调用均在执行前产生
结构化 fallback。条件必须静态为 `Bool`，不复刻 Calcit truthiness，也不使用 Calx 当前更宽松的
truthiness。

编译采用 all-or-nothing：入口可达的任意定义不合格，就不生成可执行 program，也不在 VM trap
后静默切回 Calcit。该约束有意区别于当前 Calcit Wasm emitter 对部分非导出函数生成默认值 body
的兼容策略。

## 2. 为什么 Calcit 适合产生 Calx target

Calcit frontend 已经提供第二编译目标所需的关键结构：

- `CompiledProgram`/`CompiledDef` 保存 `preprocessed_code`、稳定 `DefId`、`deps`、`schema` 和源码；
- 预处理结果已经把 macro、method 和 import 解析为比原始 Cirru 更明确的节点；
- `CalcitFn` 保存 fixed/rest arity、参数类型与返回类型；
- 现有 Wasm codegen 已证明可以从 compiled snapshot 提取函数并进行直接 codegen；
- source location、namespace/definition 和类型告警足以支撑结构化 fallback。

因此 Calcit 适合作为 frontend；主要工作不在重新解析 Cirru，而在冻结可证明的语义交集、补齐
Calx 所需指令、提供不依赖文本的 builder，并建立 differential corpus。Calx 不应反向依赖
Calcit，也不应复制 macro expansion、name resolution 或 Calcit 类型推导。

## 3. 目标与非目标

### 3.1 目标

- 自动编译至少三个真实 Calcit scalar kernel：bounded sum、Fibonacci/tail recursion、
  polynomial/affine transform；
- 对整个 reachable call closure 证明 fixed arity、具体类型和受支持语法；
- 在 VM 执行前确定 success 或 fallback，不生成 placeholder 或部分 program；
- 用版本化 ABI 描述 Calcit/Calx 参数、结果、ownership、实例生命周期和 host capability；
- 把 frontend、program construction、validation/lowering、boundary conversion、VM setup 和纯执行
  分开计量；
- 保持 Calcit runner 和 Wasm backend 的默认行为不变。

### 3.2 非目标

- 完整 Calcit 值模型、truthiness、persistent collection、nominal struct/enum 或 trait dispatch；
- macro、动态 eval、closure、任意函数值、HOF、rest/optional 参数或通用 FFI；
- 自动把任意 Calcit 函数路由到 Calx，或在 runtime trap 后透明重试；
- 在 correctness 阶段承诺加速倍数；
- JIT、GC、线程、SIMD、binary container 或完整 Wasm 兼容。

## 4. 架构与依赖方向

```text
Cirru source
  -> Calcit macro expansion / symbol resolution / type preprocessing
  -> CompiledProgram snapshot
  -> Calx eligibility + closed-call-graph extraction
       -> FallbackReport
       -> ProgramBuilder
          -> CalxProgram
          -> ValidatedProgram
          -> bound host imports
          -> CalxVM::run_typed
          -> Calcit boundary result
```

依赖方向固定为：

```text
Calcit experimental adapter -> calx_vm
calx_vm -X-> Calcit
```

初始实现应位于 Calcit 仓库的实验 backend/library integration 中。`calx_vm` 只提供中立的 strict
program、builder、validator 和 runtime API。translator 不从原始 Cirru 文本工作，也不通过
拼接 Calx Cirru 文本调用 parser。

## 5. 编译单元与 eligibility

调用方用 `(namespace, definition)` 指定一个入口。编译器按以下固定阶段执行：

1. 获取同一版本的 `CompiledProgram` snapshot，解析入口的 `DefId`；
2. 只沿 symbol-resolved direct function imports/calls 遍历，构建确定排序的 reachable call graph；
3. 对每个可达函数检查 schema、fixed arity、参数/结果/local 类型；
4. 深度遍历每个函数体，检查语法白名单、尾位置和 import capability；
5. 收集所有问题并按 `(namespace, definition, source location, code)` 稳定排序；
6. 有任一问题则返回 `FallbackReport`，不调用 builder；
7. 全部合格才构建完整 `CalxProgram`，再交给唯一 strict validation API；
8. validation 或 host binding 失败属于 compiler/integration error，仍不得执行部分 program。

`CompiledDef.deps` 可作为遍历候选和缓存失效依据，但不能单独证明可编译性：依赖列表可能包含
仅用于类型或宏处理的定义，实际 direct call edge 必须从 preprocessed expression 确认。

### 5.1 闭包和尾调用规则

- 只允许 target 已解析到 top-level `CompiledDefKind::Fn` 的直接调用；
- callee 必须 fixed arity，实参数量与已解析签名完全一致；
- 普通 direct call 降为 `Call`；
- 仅当调用位于当前函数的尾位置，才允许降为 `ReturnCall`；自递归和互递归规则相同；
- `recur` 只在当前函数尾位置、参数数量和类型完全一致时接受；
- function local、closure capture、动态 operator、trait/method 动态派发和 HOF 一律 fallback；
- 不静态证明终止。首批 fixture 的“bounded”是 corpus/precondition 与 benchmark step budget，
  不是编译器声称解决 termination checking。

## 6. 类型映射

| Calcit annotation/value | Calx boundary/storage | 首版规则 |
| --- | --- | --- |
| `Number` / `Calcit::Number(f64)` | `F64` / `Calx::F64` | 唯一普通数值映射；不根据字面量形状推断 `I64` |
| `Bool` / `Calcit::Bool` | `Bool` / `Calx::Bool` | 参数、local、结果和表达式均可用 |
| `Unit` | zero result / `CalxRunResult::Void` | 只允许作函数结果或 effect-only expression 的无值结果，不建 slot |
| `Nil` | 无 | 直接 fallback；不作为 absence、未初始化或 void sentinel |
| `Dynamic`、`Optional<T>`、`JsNullish<T>` | 无 | 直接 fallback，即使本次运行值碰巧非 nil |
| `I64` | 无 Calcit source mapping | Calx 内部仍支持；等待 Calcit 明确 Int 类型或显式 intrinsic |
| `String`、`Symbol`、`Tag` | 无 | 首版 scalar kernel 排除，避免 ownership/比较语义扩张 |
| `List<T>`、Map、Set、Buffer、BufList | 无 | 等待单独的 homogeneous typed-buffer ABI |
| Struct/Enum/Option/Result/Trait/Ref/JS object | 无 | 等待 nominal layout、ownership 和 capability RFC |

额外规则：

- 所有入口参数、入口结果、可达函数签名与 local 必须为上表的 concrete type；
- strict program 中 `Dynamic` 和 strict boundary `Nil` 计数必须都是零；
- 泛型、type variable 或 trait bound 必须先在 Calcit 侧实例化为具体签名，否则 fallback；
- `Unit` 不可赋给 local、作为参数传递或放入数据结构；
- 首版排除 source global、atom/ref 与 top-level mutable state，使 program 可重复调用且不隐含
  Calcit hot-reload state。

### 6.1 数值语义

Calcit `Number` 的运行表示是 `f64`，因此 literal、参数、local 和 result 都保持 IEEE-754 bit
语义，不执行整数化。初始映射为：

| Calcit proc | Calx lowering |
| --- | --- |
| `&+` | `Add` |
| `&- a b` | `a; b; Neg; Add` |
| unary `&- a` | `a; Neg` |
| `&*` | `Mul` |
| `&/` | `Div` |
| `&=` | 待新增 `F64Eq` |
| `&<` | 待新增 `F64Lt` |
| `&>` | 待新增 `F64Gt` |

`<=`、`>=`、`not=` 只有在 Calcit preprocessing 将其解析为已确认的 native numeric operation，
且 Calx 提供对应 `F64Le/F64Ge/F64Ne` 后才开放。禁止通过转换到 `I64`、调用动态相等、Calx
truthiness 或 host import 模拟比较。

当前 Calx 缺少全部 F64 comparison instruction；因此 #45 是 #38 在 range/Fibonacci fixture 前的
明确 VM prerequisite：parser、syntax、validator、lowering、interpreter、instruction matrix 和
正负测试必须一起增加 `f.eq/ne/lt/le/gt/ge`（最终 opcode spelling 由实现 issue 冻结）。减法不
需要独立 opcode。

NaN、正负无穷和 `-0.0` 纳入 differential corpus。只有 Calcit native proc 与 Rust `f64`
比较/算术结果逐项一致后才标记 expected same；否则该输入先标为 unsupported，不能归入普通
fallback 或静默规范化。

## 7. 表达式与控制流映射

| Preprocessed Calcit form | 状态 | Calx 规则 |
| --- | --- | --- |
| Number/Bool literal | 支持 | `Const(F64/Bool)` |
| typed local reference | 支持 | `LocalGet(index)` |
| `&let`/已降低 local binding | 支持 | 先声明 typed local，再计算值并 `LocalSet` |
| sequence / `do` | 支持 | 非末尾 value 结果显式 `Drop`；Unit expression 不压栈 |
| `if` | 支持 | condition 必须为 Bool；两分支结果类型完全一致 |
| direct named call | 支持 | `Call`，按 Calx stack order 计算参数 |
| tail direct call / `recur` | 支持 | `ReturnCall`，必须处于尾位置且签名完全一致 |
| canonical tail-recursive loop | 支持 | 优先保持 `ReturnCall`；不要求改写为 Calx `loop/br` |
| explicit approved host import | 支持 | `CallImport`，签名与 capability 必须在 allowlist |
| global/ref/atom access | fallback | 首版无跨调用状态语义 |
| closure/function value/HOF | fallback | 无 closure layout 或 indirect call |
| rest/optional call | fallback | 无稳定 strict ABI |
| Nil/collection/nominal value | fallback | 首版无 storage mapping |
| raw JS/native FFI/dynamic eval | fallback | 不属于 approved typed import |

### 7.1 求值顺序和 Bool 条件

参数与子表达式保持 Calcit 的 eager、从左到右求值顺序。即使表达式纯数值，也不能为便利而
重排可能含 approved effect import 的节点。

Calcit 中只有 `Nil`、`Unit` 和 `Bool(false)` 为 false，`Number(0)` 仍为 true；Calx 当前把
零数值视为 false。两者不等价。因此 translator 只接受静态类型为 `Bool` 的 `if`/conditional
branch，并在 builder 中生成 Bool 条件。Calx strict profile 的 validator 应在后续收紧 typed
compiler 产生的 `If/BrIf` condition；实现可以通过 builder 的 typed condition API 或独立 compiler
profile 完成，不能改变 legacy profile 已记录的 truthiness，也不能依赖 runtime `Calx::truthy()`。

### 7.2 Void 与分支结果

- `Unit` 函数使用零结果签名，运行结果为 `CalxRunResult::Void`；
- value 函数首版恰有一个 `F64` 或 `Bool` 结果；
- `if` 两分支都为 Unit 时结果为零；都为同一 scalar type 时结果为一；其他组合 fallback；
- 缺少 else 的 `if` 可能产生 Nil，因此首版 fallback；
- effect import 返回 void，不构造 `Calx::Nil`。

## 8. 宿主 ABI

ABI edition 固定为字符串 `calcit-calx-kernel/1`。edition 是 compiler adapter 与 runtime
integration 的握手字段，不等同于 crate semver。相同 edition 内只允许向后兼容地增加
diagnostic metadata；任何类型映射、ownership、调用约定或 capability 变化都必须提升 edition。

### 8.1 Entry ABI

- 入口是 fixed-arity top-level function；参数只允许 Number/Bool；
- adapter 在 VM 外把每个 Calcit scalar 按值转换为 `Calx::F64/Bool`；
- 结果只允许 void 或一个 F64/Bool，再按值转换回 `Calcit::Unit/Number/Bool`；
- Nil 不表示 void，错误也不编码成 Calcit value；compile fallback、validation/binding error 和
  runtime trap 是三个独立的 `Result` 阶段；
- scalar conversion 不共享引用、不产生 guest 可见 borrowed pointer，caller 保留原 Calcit value；
- program/validated lowering 可缓存；每次调用必须重置 operand stack、frames 和临时 locals。
  首版没有 source globals，因此无跨调用 guest state；VM 实例可在线程内复用，不声明跨线程
  `Send/Sync`。

### 8.2 Host import ABI

每个 import 声明：

```text
HostImportV1 {
  name,
  params: [F64 | Bool],
  result: Void | F64 | Bool,
  capability: Pure | Effect,
}
```

规则：

- 只支持同步、fixed arity、零或一个结果；
- guest declaration、adapter allowlist 与 Rust binding signature 必须逐项相等；
- `Pure` 才能进入可重排/缓存的未来优化，首版 translator 仍保持源码顺序；
- `Effect` 必须显式启用，不因同名函数存在而获得能力；
- host callback 的真实结果在每次返回时检查，void 使用 `Result<(), CalxError>`；
- host error 作为 host-boundary diagnostic 返回，不转换为 Nil 或 runtime fallback；
- callback 不持有 VM stack/local 引用，不允许 re-entrant guest call；异步、closure callback 和
  多结果留待后续 ABI edition。

## 9. 结构化 fallback 与错误分层

建议 Calcit adapter 暴露以下逻辑模型；Rust 名称可以调整，字段语义不可丢失：

```text
FallbackReport {
  abi_edition,
  entry: { namespace, definition },
  issues: [FallbackIssue],
}

FallbackIssue {
  code,
  namespace,
  definition,
  source_location?,
  call_path: [{ namespace, definition }],
  message,
}
```

首批稳定 code：

| Code | 含义 |
| --- | --- |
| `CALX_SUBSET_DYNAMIC_TYPE` | signature/local/expression 仍为 Dynamic |
| `CALX_SUBSET_NIL_VALUE` | Nil、Optional 或可能产生 Nil 的 form |
| `CALX_SUBSET_UNSUPPORTED_TYPE` | concrete 但首版无映射的类型 |
| `CALX_SUBSET_UNSUPPORTED_FORM` | 不在语法白名单内 |
| `CALX_SUBSET_INDIRECT_CALL` | closure、function value、HOF 或动态 operator |
| `CALX_SUBSET_ARITY` | rest/optional 或 fixed arity 不匹配 |
| `CALX_SUBSET_NON_BOOL_CONDITION` | condition 未静态证明为 Bool |
| `CALX_SUBSET_RECUR_NOT_TAIL` | recur/tail candidate 不在尾位置 |
| `CALX_SUBSET_HOST_CAPABILITY` | import 未声明、未批准或 signature 不匹配 |
| `CALX_SUBSET_CALL_CLOSURE` | 入口因可达 callee 不合格而整体回退 |
| `CALX_SUBSET_ABI_EDITION` | compiler/runtime adapter edition 不兼容 |

编译 fallback 不是 Calx runtime trap。错误分层固定为：

1. `FallbackReport`：程序不属于子集，可由调用方明确选择 Calcit runner；
2. build/validation/binding diagnostic：translator 或集成契约错误，不能降级掩盖；
3. `CalxError` runtime trap：已验证 program 执行失败，返回调用方，不自动重跑 Calcit。

## 10. Golden kernel 与 correctness corpus

首批三个必须完成的 source-backed fixture：

1. `range-sum`：Number 参数、typed locals、Bool numeric comparison、tail recursion；
2. `fibonacci`：direct recursion、两个 direct calls、if 和 F64 comparison；输入 wrapper 限定为
   有限、非负、整数值的 Number；
3. `affine` 或 polynomial transform：多参数算术、local binding、direct helper call。

建议第四个 fixture 是 fixed-step scalar simulation，用显式剩余步数递减，覆盖更深 local 与
tail-call 状态。typed buffer 尚未定义前，不用普通 Calcit List 伪装 dot product 输入。

每个 fixture 保存并检查：

- Calcit source 与 resolved entry；
- typed preprocessed form 的稳定摘要；
- generated `CalxProgram`/builder snapshot；
- validation/lowering report；
- Calcit runner 与 Calx 的 input/output corpus；
- expected trap、fallback code、source location 和 call path；
- Calcit、calx_vm 与 toolchain version。

边界 corpus 至少覆盖 false/true、`0.0`、`-0.0`、负数、非整数、NaN、正负无穷、最小/最大
fixture 输入、深递归/step budget，以及 unsupported callee 使整个 closure fallback。

## 11. 性能计量契约

#39 只能在 correctness corpus 通过后采样，并分别报告：

1. Calcit preprocessing/snapshot 获取；
2. eligibility 与 reachable graph；
3. ProgramBuilder construction；
4. validation/lowering；
5. host binding 与 VM setup；
6. Calcit -> Calx 参数转换；
7. pure VM execution；
8. Calx -> Calcit 结果转换；
9. 端到端总耗时。

至少覆盖 cold one-shot、缓存 program 后的 one-shot、重复 hot call 和多个输入规模，并报告
crossover point。不能从现有手写 Calx demo 的纯 interpreter microbenchmark 推导 Calcit
端到端收益；也不能隐藏较慢或 fallback 的 kernel。

## 12. 后续 typed buffer 方向

标量闭环完成前不设计容器。后续如真实任务证明边界转换是主要瓶颈，单独 RFC 可定义
`F64Buffer`（优先）及可能的 `I64Buffer`：

- element type 必须同质并进入 Calx type system，不复用非泛型 `Calx::List`；
- 明确 owned/borrowed、可变性、长度、越界 trap 和 host lifetime；
- Calcit Number buffer 自然映射 F64；I64Buffer 必须有显式 source intrinsic/conversion；
- benchmark 同时报告复制、pin/borrow 和纯 loop 成本；
- 不借 buffer ABI 引入完整 persistent collection 或 GC。

## 13. 实现顺序与完成条件

依赖顺序：

1. #45 增加 F64 comparisons，并把 typed compiler condition 收紧为 Bool；
2. #37 提供 source-aware `ProgramBuilder`，只产生待验证 `CalxProgram`；
3. #38 PR A 实现 eligibility、fallback 和三个 scalar golden fixtures；
4. #38 PR B 接入真实 Calcit `CompiledProgram` snapshot 和 differential corpus；
5. #39 在 correctness 固定后采集端到端数据；
6. 只有数据证明需要批量 boundary，才启动 typed-buffer RFC。

本 RFC 完成的判据是：类型/语法表、Bool/truthiness/number/void 语义、all-or-nothing fallback、
versioned ABI、三个 kernel 的纸面映射、性能阶段边界和已知 VM prerequisite 都有唯一解释，#37
与 #38 不需要再猜测这些决策。

## 14. 被拒绝的替代方案

### 从原始 Cirru 直接翻译

这会重复 macro expansion、symbol resolution 和类型推导，并把 frontend 语义漂移隐藏在字符串
规则中。translator 必须消费 typed preprocessed representation。

### 把所有 Number 猜成 I64

Calcit 只有 `Number(f64)`，字面量没有小数点不构成 Int 类型证明；猜测 I64 会改变除法、范围、
NaN/Infinity 和 boundary ABI。

### 沿用 Calx/Calcit truthiness

两者对零数值的 truthiness 不同。只接受 Bool condition 的规则更窄，但可证明且不会随 VM legacy
行为变化。

### 部分编译并为失败函数生成默认值

placeholder 会把编译失败伪装成合法结果，并破坏 differential testing。入口闭包必须整体成功或
整体 fallback。

### runtime trap 后自动重跑 Calcit

effect import 可能已执行，自动重跑会重复副作用；它也掩盖 validator/runtime bug。fallback 只在
执行选择阶段发生。

### 首版直接支持 persistent List/Map/Set

这会立刻引入 element type、allocation、equality、ownership 和 GC/layout 问题，使 scalar
correctness 与性能信号失焦。先证明 scalar pipeline，再基于 workload 设计 typed buffer。
