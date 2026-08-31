# RFC 0004：严格 F64Buffer 类型与宿主 ABI

- 状态：实现中（#52）
- 目标版本：0.5 typed-buffer vertical slice
- 关联 Issue：#50、#51
- 前置：RFC 0002 typed boundaries、RFC 0003 Calcit subset/host ABI
- 后续实现：#52 VM/validator、#53 Calcit adapter/benchmark

## 1. 摘要

本 RFC 为 Calx strict profile 增加首个同质批量数值类型 `F64Buffer`，用于 dot product、moving
aggregate 等真实 numeric kernel。它是独立 concrete type，不复用 `Calx::List`，element 不
boxing，slot 不 nullable；函数、local 与 host import 边界继续保持 non-nil、zero-Dynamic。

首版 buffer 不可变，值语义由共享所有权实现。Calx value clone 只复制共享 handle，不逐元素
复制。VM 提供 `f64-buffer.len`、`f64.to-i64-index` 与 `f64-buffer.get`；索引是 `I64`，非法转换、
越界或负索引产生 trap，绝不返回 `Nil`。Calcit `Number` 仍只映射 `F64`，producer 必须通过显式
checked conversion 形成索引，不得根据数值字面量推断 I64。

typed-buffer kernel 使用 `calcit-calx-kernel/2` ABI edition。edition 1 保持 scalar-only，不能把
List、Buffer bytes、Nil 或 Dynamic 解释成 F64Buffer。fallback 仍只发生在 eligibility/selection
阶段；VM 开始执行后的任何 trap 都不会触发 Calcit 重跑。

## 2. 动机与边界

scalar correctness 与 benchmark 已证明 Calcit typed snapshot 可以稳定产生 Calx program，但
逐元素穿过 host import 或先构造 persistent List 都会掩盖真实批量计算收益。Calx 需要的是一个
语义窄、成本可计量的连续 F64 storage，而不是复制 Calcit collection、GC 或 resource model。

首版明确不包含：

- `I64Buffer`；只有 Calcit 出现显式 Int 类型或 intrinsic 后才另行设计；
- mutable element、guest-side grow、slice/view 或 borrowed guest lifetime；
- generic element type、nullable slot、Dynamic element 或隐式 boxing；
- 从 Calcit List/Map/Set 或 byte Buffer 的隐式转换；
- SIMD、线程、自动 offload、VM pooling、linear memory 或 WIT resource；
- buffer equality、ordering、hash、truthiness、serialization 或 constant literal syntax。

## 3. 类型与值表示

新增类型 `CalxType::F64Buffer` 与值 `Calx::F64Buffer`。文本 module type token 固定为
`f64-buffer`。它与现有 `list`、`str`、`f64` 都不兼容；strict validator 不做 structural 或
numeric coercion。

语义表示是不可变的连续 `f64` sequence。0.5 Rust 实现使用 `Rc<[f64]>` 作为单线程 VM 的共享
backing：

- clone 是 O(1) handle clone；
- `local.get`、函数参数/结果与 import 传递 clone handle，不复制元素；
- backing 在最后一个 Calx/host handle 释放后回收；
- 此选择不承诺 `Send`/`Sync`，未来若引入线程必须另开 RFC；
- diagnostic/display 只报告类型和长度，不展开巨型内容，也不扩大 `CalxError` inline payload。

公开构造 API 区分三种意图：

1. `share`：接收已有共享 backing，O(1)；
2. `adopt`：消费 host-owned `Vec<f64>`，实现可以为归一化 shared backing 重新分配或复制；
3. `copy_from_slice`：从 borrowed slice 明确复制。

`adopt` 不是 zero-copy 承诺。benchmark 必须分别记录 allocation/copy，并且不能把构造成本藏入
pure execution。VM 不存储 `&[f64]`：跨重复调用、hot reload 和 callback 生命周期的 borrowed
reference 不进入首版 ABI。

## 4. 支持边界

F64Buffer 首版允许出现在：

- strict function parameter 与单结果；
- declared local；
- typed host import parameter 与单结果；
- typed block/loop parameter 与结果，只要现有 structured-control validator 可证明一致。

F64Buffer 首版不允许 global declaration。global initializer、持久存储和 host reload ownership
需要额外生命周期决策；validator/builder 应明确拒绝，而不是退回 Dynamic。普通 scalar global
行为不变。

strict host binding 仍是信任边界：声明为 F64Buffer 的参数/结果必须在运行时再次核对实际
`Calx::F64Buffer` variant。不匹配返回 host-boundary trap，不做 List/Buffer conversion。

## 5. 指令语义

首版只增加三条指令：

| 文本指令 | typed stack effect | 语义 |
| --- | --- | --- |
| `f64-buffer.len` | `[F64Buffer] -> [I64]` | 返回 element count |
| `f64.to-i64-index` | `[F64] -> [I64]` | 显式 checked index conversion |
| `f64-buffer.get` | `[F64Buffer, I64] -> [F64]` | 读取指定 element |

`len` 在 backing 长度无法表示为 I64 时 trap。Rust 实现不得用 unchecked cast；当前平台上应使用
checked conversion。

`f64.to-i64-index` 要求输入 finite、无小数部分且位于 `0 <= n < 2^63`；`-0.0` 明确映射为
`0`。失败产生结构化 conversion trap。实现必须先按该半开区间检查，再转换为 I64；不能用
`i64::MAX as f64` 作为包含式上界，因为该转换会舍入为 `2^63`。这不是通用 truncation，也不接受
rounding、saturation 或 wrapping。

`get` 先要求 index 非负，再 checked-convert 为 `usize`，最后检查 `index < len`。任一步失败都
返回结构化 bounds trap，至少包含 instruction、index 与 length；不得 panic、wrap、clamp、返回
Nil 或读取默认 `0.0`。NaN/Infinity 只可能作为 element value，不能成为 index。

buffer 本身不可变，因此首版没有 `set`、`push`、`grow`。`drop`、local move/copy、direct call、
return 与 typed import 可按普通 concrete value 工作。buffer 不参与 arithmetic、comparison、
truthiness 或 legacy list instructions。

## 6. Validator 与 builder 契约

strict validation 必须：

- 把 F64Buffer 当作已知 concrete `ValidationType`；
- 对两条指令执行上表的精确 stack check；
- 拒绝以 List、Nil、Dynamic、F64 或 byte Buffer 替代 F64Buffer；
- 拒绝 F64Buffer global；
- 保持 unreachable/control-stack 规则与 scalar type 相同；
- 对手工构造的非法 public IR 返回 validation/program error，不 panic。

`ProgramBuilder`/`FunctionBuilder` 必须能声明 F64Buffer 参数、结果、local、block/loop 和 import，
并提供 source-aware `f64_buffer_len`、`f64_to_i64_index`、`f64_buffer_get` helper。builder 只构造
未验证 program，仍由 `ValidatedProgram` 完成最终证明。

文本 parser 识别 `f64-buffer` type token，以及 `f64-buffer.len`、`f64.to-i64-index`、
`f64-buffer.get` 三条 instruction，但不增加 buffer literal。可执行 buffer 从 typed entry argument
或 typed host import 获得；这避免在 RFC 中额外设计大常量编码。

## 7. Calcit producer 与索引转换

Calcit frontend/adapter 只有在 typed snapshot 中看到显式 F64Buffer boundary/intrinsic 时才能
lower。普通 `List Number`、`Buffer`、`JsObject`、`AnyRef` 或 Dynamic 都不满足 eligibility，且不
发生隐式逐元素转换。

Calcit `Number(f64)` 到 Calx index 的转换必须是显式 checked intrinsic，并唯一 lowering 为
`f64.to-i64-index`。它至少拒绝：

- NaN 与正负 Infinity；
- 小数；
- 负数；
- 达到或超过 `2^63` 的值。

静态类型不是 Number 时在 eligibility 阶段 fallback；Number runtime value 不满足上述条件时由
`f64.to-i64-index` 产生 runtime trap。不得从 `1.0` 的字面量形状推断 source-level I64，也不得用
Nil、Option 或默认值表示转换失败。Calcit native reference 必须以同样条件报错，不做 rounding。
转换得到合法 I64 后，目标 buffer 的实际长度仍只由 `f64-buffer.get` 检查。

Calcit native reference 与 Calx execution 必须消费相同 element sequence。若 embedding 从 Calcit
collection 显式构造 F64Buffer，该构造是用户可见 conversion，必须在 boundary 阶段计量，不能被
compiler 自动插入。

## 8. ABI edition 与兼容性

包含 F64Buffer boundary 或 intrinsic 的 kernel 必须声明 `calcit-calx-kernel/2`：

- edition 1 维持 RFC 0003 的 scalar-only contract；
- edition 1 consumer 必须拒绝 edition 2，不得忽略未知类型/instruction；
- edition 2 consumer 可以通过显式 edition 1 路径运行原 scalar kernel，但不能就地重新解释 ABI；
- edition 2 entry signature 可包含 F64Buffer/F64/Bool 与 void/single concrete result，仍禁止 Nil、
  Dynamic、optional/rest ABI；
- host capability manifest 必须包含完整 buffer signature 与 ownership mode；
- host bindings 每次 VM 实例化重新附着，不进入 source-derived compile cache。

ownership mode 至少区分 `share`、`adopt`、`copy`，并进入 benchmark metadata。它描述 host 到 VM
的构造动作，不改变 guest 内不可变共享语义。

## 9. Fallback、trap 与热替换

以下情况在 VM 执行前产生 whole-kernel fallback 或 boundary rejection：

- source type 不是显式 F64Buffer；
- 发现 Nil/Dynamic/List/byte Buffer coercion；
- ABI edition、signature 或 ownership capability 不匹配；
- call graph 中任一 reachable definition 使用未支持 buffer 操作。

program construction、validation、host binding、index conversion 与 runtime bounds trap 保持不同
错误阶段。一旦 VM 开始执行，任何 trap 都直接返回；不能自动重跑 Calcit，因为 import side effect
可能已经发生。

compile cache 只能保存 immutable source-derived artifact。F64Buffer values 与 host callbacks 是
per-use binding/input，不进入 cache；hot reload 后必须重新绑定，避免 stale data/callback。

## 10. 与 Wasm 的关系

本设计只借用 Wasm 的显式数值类型、checked boundary 和越界 trap，不声称 memory 或 binary
兼容：

- Wasm linear memory 是 byte-addressed mutable storage；F64Buffer 是 opaque、homogeneous、immutable
  value；
- F64Buffer 没有 page、byte offset、alignment、load/store、grow 或 shared-memory 语义；
- `f64.to-i64-index` 是 Calcit producer 所需的窄 checked conversion，不对应某条兼容承诺；
- #34 的 mapping 应把三条指令标为 Calx-specific，并只比较其数值与 trap 子规则。

若未来用 linear memory 取代或承载 buffer，必须另开 RFC 说明 aliasing、ownership 与 ABI 迁移，
不能把本 edition 的 opaque handle 静默解释为 address。

## 11. 测试与性能证据

#52 至少覆盖：

- type token、instruction parse 与 builder positive tests；
- stack type mismatch、F64Buffer global、legacy List/byte Buffer rejection；
- empty/single/multi-element len/get；
- NaN/Infinity/fractional/negative/overflow conversion trap；
- equal-to-len 与 greater-than-len bounds trap；
- typed import 参数/结果实际 variant mismatch；
- clone/share 生命周期与旧 scalar/legacy regression；
- malformed public IR 不 panic。

#53 至少覆盖一个真实 source-backed dot product 或等价 aggregate，并固定 Calcit reference、generated
program、fallback 与 trap goldens。benchmark 必须分别报告：

1. source frontend/snapshot/eligibility；
2. lowering plan/program construction/validation；
3. buffer allocation 与 adopt/share/copy；
4. host binding 与 VM setup；
5. pure execution；
6. result conversion 与 total time；
7. 输入规模 crossover、样本/warm-up/noise metadata。

普通 CI 只做 correctness 与 schema checks，不设置噪声敏感的绝对性能阈值。

## 12. 被拒绝的替代方案

### 复用 `Calx::List`

List 没有 homogeneous element contract，会把每次读取重新变成 tag check，并允许 Nil/Dynamic 混入，
与 strict typed-buffer 目标冲突。

### 复用 Calcit byte Buffer

byte Buffer 没有 F64 alignment、endianness 和 element identity；隐式 reinterpret 会制造跨 backend
差异。显式编码格式未来可以作为 conversion 输入，但不是 F64Buffer 自身。

### 首版保存 borrowed slice

它会把 Rust lifetime、VM 重复调用、hot reload 与 callback retain 绑在一起，且难以在现有 owned
`Calx` enum 中表达。首版选择 owned shared backing。

### 使用 F64 index

允许 NaN、小数和 Infinity 进入基本 indexing opcode，会把 Calcit source convenience 泄漏到 VM
type system。VM 使用 I64，Calcit adapter 负责显式 checked conversion。

### 越界返回 Nil 或 Option

Nil 会重新引入 sentinel；Option 会把简单的 VM safety fault 变成新增 guest sum type，并扩大首版
控制流与 ABI。首版统一 trap。

## 13. 完成条件

本 RFC 完成的判据是：F64Buffer 的 concrete type、共享不可变 storage、支持边界、三条指令、
I64 index、conversion/bounds trap、producer restriction、ABI edition、fallback 与计量阶段均有唯一
解释；#52 与 #53 可以按此实现，不再自行决定 Nil/Dynamic、ownership 或 conversion 语义。
