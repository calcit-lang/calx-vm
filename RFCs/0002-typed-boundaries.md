# RFC 0002：类型化 Local、Global 与 Import 边界 / Typed Local, Global, and Import Boundaries

- 状态：已实现 / Implemented
- 目标版本：0.3
- 关联 Issue：#31
- 前置：RFC 0001、#30 source-aware diagnostics

## 摘要 / Summary

Calx 将增加显式 `CalxProgram` 模块元数据，用于声明函数内 local、模块 global 与宿主
function import 的类型契约。面向性能的 strict typed profile 要求所有声明都是非 `nil` 的
`Known(CalxType)`；`Dynamic` 只保留为旧源码与旧 embedding API 的兼容标记，不得进入严格
模块。VM 也不再使用 `nil` 表示“缺失”“未初始化”或 void。

The proposal adds explicit `CalxProgram` metadata for function locals, module
globals, and host function imports. The performance-oriented strict typed
profile only admits non-`nil` `Known(CalxType)` declarations. `Dynamic` remains
an explicit marker for legacy source and embedding APIs, but is rejected from
strict modules. The VM also stops using `nil` as a missing, uninitialized, or
void sentinel.

本 RFC 的实现按三个 PR 拆分：

1. RFC 与文档索引；
2. program representation、Rust API 与 parser；
3. validator、`ValidatedProgram`、runtime、CLI 与兼容回归。

## 背景与问题

RFC 0001 的单遍 validator 已能证明函数参数、常量、函数调用和结构化控制流的已知类型，
但以下边界仍不完整：

- `local.new` 是运行期指令，只产生 `Dynamic`，并可能随控制路径改变 local layout；
- 初始 globals 由 `CalxVM::new` 的实际值临时推断，`global.new` 仍按执行顺序分配，且没有
  mutability；
- `CalxImportsDict` 只记录 callback 与 arity，validator 把所有参数和固定单结果视为
  `Dynamic`；
- parser 返回函数列表，尚无承载 global/import 声明的 module metadata；
- 旧 API 的动态保证容易被误读为“验证通过即类型安全”。

因此必须先把“模块声明”“宿主绑定”“VM 实例状态”分开，再扩大静态保证。

## 目标

- local/global/import 的已知类型错误在 lowering 和执行前失败；
- local index、global index 与 import name 在模块上下文中稳定解析；
- global mutability 与 initializer 类型可验证；
- typed import 的 guest call 和 host binding signature 均在执行前检查；
- host callback 的真实返回值继续在运行时守卫，不能从 Rust function pointer 猜测正确性；
- strict typed profile 中 `Dynamic` 数量必须为零；legacy `Dynamic` 在 Rust metadata、
  `check`/`explain` 中都可观察；
- `nil` 只表示源码显式构造的语言值，不承担未初始化 slot、缺失返回值或 VM 状态哨兵职责；
- strict typed local/global/import 以及函数参数/结果不接受 `nil` 类型；void 使用零结果表达；
- 旧 demos 与 `CalxVM::new(fns, globals, imports)` 保持兼容并有迁移路径。

## 非目标

- 不增加 memory、table、GC、持久化集合或 global import；
- 不设计 async/closure host callback；
- 不在 0.3 支持 host import 的多返回值；零或一个返回值均受支持；
- 不实现 Calcit → Calx 编译器；
- 不删除 public `CalxSyntax::LocalNew/GlobalNew` 或旧 import tuple；
- 不从 `Calx` 删除显式 `Nil` value；本 RFC 只禁止把它当隐式 VM 状态和严格边界类型；
- 不把 declaration、validation、runtime 和 host error 合并为一个大型错误对象。

## 类型契约

parser 与兼容诊断使用独立于 validator 内部栈状态的公开边界类型：

```rust
pub enum CalxBoundaryType {
  Known(CalxType),
  Dynamic,
}
```

规则：

- `Known(T)` 只接受运行值或静态栈值 `T`；strict profile 进一步拒绝 `Known(Nil)`；
- `Dynamic` 表示静态信息不足，不表示任意值已经被证明正确；
- `Known` 与 `Dynamic` 的匹配沿用 RFC 0001：验证可以继续，但消费者保留运行时检查；
- `Dynamic` 不加入 `CalxType`，避免把“未知”伪装成运行值类型；strict Cirru declaration
  不接受 `dynamic` token，只有 legacy adapter 可以产生该 variant；
- `CalxProgram` 的 declaration validation 要求 dynamic boundary count 为零；手工构造的
  `CalxBoundaryType::Dynamic` 也由 `CalxVM::from_program` 拒绝；
- `ValidationType` 在 validator 内部继续表示操作数栈状态；实现阶段可与
  `CalxBoundaryType` 相互转换，但两者职责不同。

### `nil` 与 `Dynamic` 使用预算

两者都必须有窄而可审计的来源：

- `nil` 可由显式 `const nil` 或 legacy host callback 产生，也可能作为 `list` 中的显式数据；
- `nil` 不得作为 local/global 的默认值、未初始化 marker、void return 或 VM 尚未结束的 marker；
- `Dynamic` 只能由 `local.new`、`global.new`、旧 tuple import 或旧 embedding API 合成；
- strict parser/builder、`CalxProgram` 和 typed host binding 不提供创建 Dynamic 合同的正常路径；
- fully typed program 验证完成后，local/global 的热路径不应重复做动态类型检查；host callback
  仍是信任边界，返回值检查不得省略；
- legacy profile 必须显式标记，不能因“运行成功”被升级成 fully typed。

这不是只影响诊断的风格约束。Dynamic 会阻止 validator 为热路径建立稳定类型假设，隐式
`nil` 则会把未初始化错误推迟到执行中；两者都直接削弱 VM 作为 Calcit typed subset 执行层的
性能与可优化性。

## Cirru 源码语法

### Typed local declaration

local declaration 位于函数签名之后、第一条可执行指令之前，不生成 `CalxSyntax`：

```cirru
fn accumulate (($input i64) -> i64)
  local $sum i64
  local $done bool
  local.get $input
  local.set $sum
  local.get $sum
  return
```

约束：

- 形式固定为 `local $name TYPE`；name 必须唯一；
- 参数 index 在前，声明 local 按源码顺序在后，与 Wasm local index space 一致；
- 所有 local 均可通过 `local.set/tee` 修改，类型本身不可改变；
- known local 在每次函数调用建 frame 时使用非 nil 类型默认值初始化：`bool → false`、
  `i64 → 0`、`f64 → 0.0`、`str → ""`、`list → []`；实现应复用不可变空值，避免每次建
  frame 产生不必要分配；
- `nil`、`dynamic` 和尚无运行值的 `link` 均不得声明为 strict local type；
- 声明不得出现在 block/loop/if 内，也不得出现在可执行指令之后。

旧源码 `local.new` 只在函数 declaration prefix 中兼容：

```cirru
fn legacy ()
  local.new $value
  ;; legacy-only uninitialized Dynamic slot
```

无 name 的 `local.new` 使用下一个数字 local name。parser 将 source-level legacy form
规范化为 function-entry legacy Dynamic declaration，不再让控制流决定 local layout。它不能
被 `ParsedProgram::into_program()` 提升为 strict program。slot 初始状态为 `Uninitialized`，
`local.get` 在首次 `local.set/tee` 前 trap；实现不得用 `Calx::Nil` 编码该状态。出现在执行区或
嵌套控制结构中的 `local.new` 将产生 parse diagnostic。手工构造的
`CalxSyntax::LocalNew` 仍只由 legacy executor 接受，属于 Rust IR compatibility escape hatch。

### Typed global declaration

global 是 top-level module declaration：

```cirru
global $counter (mut i64) 0
global $build-id (const str) "|dev build"

fn main (-> i64)
  global.get $counter
  return
```

约束：

- 形式固定为 `global $name (MUTABILITY TYPE) INITIALIZER`；
- mutability 只接受 `const` 或 `mut`；
- initializer 使用当前 `const` 标量 literal 规则，`Known(T)` 必须与 initializer 类型一致；
- `nil`、`dynamic` 与 `link` 不得作为 strict global type；initializer 因此不得为 `nil`；
- global index 按 declaration 出现顺序固定；`global.get/set` 同时接受 `$name` 和旧数字 index；
- `global.set` 对 `const` global 总是在验证阶段失败；对 `mut Known(T)` 检查 `T`；
- declarations 可与 functions 交错书写，但 parser 先做 module pre-pass，因此所有函数看到
  同一个完整 global context；duplicate name 在 parse/module validation 阶段失败。

旧 `CalxVM::new` 传入的非 nil globals 映射为匿名、`mut Known(actual.value_type())` legacy
声明；传入 `Calx::Nil` 的 global 保持 legacy Dynamic，不能进入 strict program。旧
`global.new` 只追加匿名、未初始化的 legacy Dynamic slot；`global.get` 在首次 set 前 trap，
实现不得以 `Calx::Nil` 作为占位。它不能建立可供其他函数静态引用的 module declaration。
新源码和 builder 不应依赖跨函数的 `global.new` 顺序。

### Typed function import declaration

0.3 只定义单名、同步、零或单结果 function import：

```cirru
import-fn add2 (i64 i64 -> i64)
import-fn log2 (i64 i64 ->)

fn main (-> i64)
  const 20
  const 22
  call-import add2
  return
```

约束：

- 形式固定为 `import-fn NAME (PARAMS... -> [RESULT])`；0.3 接受零或一个 result；
- name 在 Calx module 中唯一，并继续由 `call-import NAME` 引用；
- validator 按逆栈顺序消费 params，随后压入 result；
- params 与可选 result 必须是非 nil concrete types；strict source 不接受 `dynamic`；
- source declaration 与 host binding 宣告的 signature 必须在 VM 实例化、执行任何 guest
  指令之前完全一致；
- callback 是不透明宿主代码。value callback 返回后 VM 仍检查真实类型；void callback 使用
  `Result<(), CalxError>`，不构造 `Calx::Nil`。callback kind/signature 违反声明时返回 host
  boundary error，而不是把错误归因于 guest validator；
- 旧源码可省略 import declaration。旧 tuple `(callback, arity)` 会被提升为
  `Dynamic × arity -> Dynamic`，因此现有 demos 保持运行但不声称静态安全。

## Rust 表示与 API

第二阶段实现采用以下逻辑模型；字段可以使用 `Rc<Vec<_>>` 等现有共享布局，但语义不得改变：

```rust
pub struct CalxLocalDecl {
  pub name: Rc<str>,
  pub value_type: CalxBoundaryType,
  pub span: Option<SourceSpan>,
}

pub enum CalxMutability {
  Const,
  Mut,
}

pub struct CalxGlobalDecl {
  pub name: Rc<str>,
  pub value_type: CalxBoundaryType,
  pub mutability: CalxMutability,
  pub initial: Calx,
  pub span: Option<SourceSpan>,
}

pub struct CalxImportDecl {
  pub name: Rc<str>,
  pub params: Rc<Vec<CalxBoundaryType>>,
  pub result: Option<CalxBoundaryType>,
  pub span: Option<SourceSpan>,
}

pub struct CalxProgram {
  functions: Vec<CalxFunc>,
  globals: Vec<CalxGlobalDecl>,
  imports: Vec<CalxImportDecl>,
}
```

`CalxProgram` 字段由 strict constructor 验证后保持私有，只通过只读 accessor 与 consuming
`into_parts()` 暴露，防止构造后重新注入 Dynamic/Nil boundary。

`CalxFunc` 增加非参数 `locals: Rc<Vec<CalxLocalDecl>>`。`local_names` 暂时保留以兼容
debug/display consumer；实现应提供 constructor/builder，避免继续要求下游手写不断增长的
struct literal。

`CalxBoundaryType` 保留在表示中是为了让 parser、legacy adapter 与诊断不丢失来源；
`CalxProgram` 的 strict validation 只接受其中的 `Known(non_nil_type)`。宿主绑定与 guest
declaration 分离，并在 Rust 类型上区分 void/value callback：

```rust
pub enum CalxHostCallback {
  Void(fn(&[Calx]) -> Result<(), CalxError>),
  Value(fn(&[Calx]) -> Result<Calx, CalxError>),
}

pub struct CalxHostBinding {
  callback: CalxHostCallback,
  params: Rc<Vec<CalxType>>,
  result: Option<CalxType>,
}

pub type CalxHostBindings = HashMap<Rc<str>, CalxHostBinding>;
```

typed host binding 直接保存 concrete `CalxType`，因此 Rust 正常构造路径无法表达 Dynamic；
`CalxHostBinding::void/value` 返回 `Result` 并拒绝 `Nil`/`Link`。parsed declaration 仍使用
`CalxBoundaryType`，使 legacy Dynamic 在 strict conversion 失败前保持可观察。

新入口：

```rust
CalxVM::from_program(program: CalxProgram, bindings: CalxHostBindings)
  -> Result<CalxVM, CalxProgramError>
```

typed execution 不用 `Calx::Nil` 表示 void：

```rust
pub enum CalxRunResult {
  Void,
  Value(Calx),
}

CalxVM::run_typed(args: Vec<Calx>) -> Result<CalxRunResult, CalxError>
```

strict VM 内部的完成状态同样使用独立 control state，而不是预先把 `return_value` 设为
`Calx::Nil`。旧 `run() -> Result<Calx, CalxError>` 继续把 `Void` 映射为 `Calx::Nil`，但该
转换只存在于 legacy API adapter，不能渗入 typed frame、operand stack 或 host ABI。

旧入口继续保留：

```rust
CalxVM::new(fns, globals, imports: CalxImportsDict) -> CalxVM
```

旧 `fn(&Vec<Calx>)` callback 和 tuple arity 由单独 legacy binding variant/adapter 保存，
不能假定它自动满足新 `fn(&[Calx])` typed ABI。0.3 文档标记旧 alias 为 legacy，但不立即加
Rust `#[deprecated]`，避免严格 `-D warnings` 让仓库自身和下游无法渐进迁移。

`parse_program` 的兼容表示固定为：

```rust
pub struct ParsedProgram {
  /// 只包含 function AST，继续与 `functions` 按 index 平行。
  pub nodes: Vec<Cirru>,
  pub functions: Vec<CalxFunc>,
  pub globals: Vec<CalxGlobalDecl>,
  pub imports: Vec<CalxImportDecl>,
}
```

top-level declaration nodes 不混入 `nodes`，避免破坏 `calx explain` 和 #30 consumer 的
function index 对应关系。`ParsedProgram::into_program()` 移动 functions/globals/imports 生成
`CalxProgram`；遇到 legacy Dynamic 或 nil-typed boundary 时返回 `CalxProgramError`，而不是
静默降级。AST-only `parse_function` 继续返回 legacy function，source spans 与 module
declarations 仍不可用。

representation/parser 阶段采用两遍 module parsing：第一遍收集 global/import declaration 与
稳定 index，第二遍解析 functions，因此 named global 可以前向引用。`ParsedProgram.nodes`
只保留 function AST；`dynamic_boundary_count()` 暴露 legacy 边界数量，`into_program()` 执行
strict conversion。

### Profile 选择

- 含 typed local/global/import declaration 的 `calx run/check/explain` 走 strict profile；
- 旧源码继续通过独立 legacy CLI adapter 或 `CalxVM::new` / `parse_function` API 运行；
- 兼容的一参数 CLI alias 继续识别旧调用；
- strict profile 不做“发现 Dynamic 后自动重试 legacy”的 fallback；这类 fallback 会掩盖
  性能退化并让 CI 无法阻止新的动态边界；
- implementation PR 在切换 CLI 默认值前先迁移仓库 demos，因此普通示例继续走 strict path。

## Module validation 与实例化

处理顺序固定为：

```text
Cirru declarations + functions
  -> parse/module pre-pass
CalxProgram
  -> strict declaration validation (zero Dynamic, zero nil-typed boundary)
  -> function validation
ValidatedProgram + host bindings
  -> binding signature validation
  -> globals/frame instantiation
CalxInstr
  -> runtime + dynamic/host guards
```

### Declaration validation

在函数验证前完成：

- local/global/import name 唯一性和 type token 合法性；
- strict local/global/import 以及 function params/results 均拒绝 `Dynamic` 与 `Nil`；
- known global initializer 类型；
- import 具有零或一个 result，callback kind 与 result arity 一致；
- `link` declaration 拒绝；
- named/numeric global index 可解析；
- legacy declaration 显式标记为 Dynamic，且不能进入 strict instantiation。

### Function validation

validator context 改为：

```text
strict locals  = Known(function params) ++ Known(function local declarations)
strict globals = Known(module global contracts)
strict imports = Known(module import signatures)
```

legacy validation 使用独立 context，把旧 runtime globals 和 tuple imports 显式合成为 Dynamic；
不得把两套 context 合并后再把整个 program 标记为 typed。

规则：

- `local.get/global.get` 压入对应 contract；
- `local.set/tee` 与 mutable `global.set` 使用 RFC 0001 matching rule；
- immutable `global.set` 在 pop 之前失败，保证 error stack 不被部分修改；
- typed `call-import` 精确消费参数并压入 result；
- unknown index/name、duplicate declaration、known mismatch 均在 lowering 前失败；
- legacy context 中经过 Dynamic 的操作在 trace 中保持 Dynamic，不得升级为 Known；
- strict context 验证完成后必须保持 zero Dynamic invariant，供 lowering/runtime 选择无动态
  local/global type guard 的快路径。

### Host binding validation 与 runtime guards

`CalxVM::from_program` 在创建可执行 VM 前检查：

- 每个 declared import 恰有一个 binding；
- params/result contract 与 declaration 完全相等；
- `Void` callback 只匹配无 result declaration，`Value` callback 只匹配一个 result；
- host 不得提供 module 未声明的额外 bindings，保持 capability boundary 可审计；
- strict `from_program` 不从 bindings 猜测缺失的 source declaration，undeclared
  `call-import` 在 module validation 阶段失败；
- 只有旧 `validate_program` / `CalxVM::new` compatibility path 才从
  `CalxImportsDict` 的 arity 合成 Dynamic contracts。未迁移的 legacy demos 继续走该路径。

callback 执行前仍检查实参数量和所有参数的运行值类型，value callback 返回后检查
`Known(result)`；void callback 不向 operand stack 压值。这是宿主信任边界，不因 guest
validation 成功而删除。callback 自己返回的 `CalxError::new_raw` 及 signature violation
都属于 host phase，不伪造 VM snapshot。

## Legacy Dynamic 的可观察性与默认拒绝

Dynamic 不是 warning-free 的静态成功，更不是 strict program 的合法 contract。实现必须同时
提供：

- Rust metadata 中显式的 `CalxBoundaryType::Dynamic`；
- `trace_validation` / `calx explain` 中现有的 `Dynamic` 栈显示；
- legacy `calx check` 摘要中的 dynamic boundary 数量；
- source-aware explain 输出中每个 dynamic local/global/import declaration 的位置；
- runtime mismatch 继续使用实际指令 source span 或 host boundary message。

`calx check` 的 typed profile 默认遇到 Dynamic 即失败。兼容旧源码需要显式进入 legacy
profile，诊断和 stderr 必须标出 Dynamic 数量；不得提供一个静默的全局 `allow dynamic`
开关给新 module/builder。Calcit 调用边界应逐项提供 concrete signature；尚未迁移的调用
只能留在较慢的 legacy executor。

## 错误与诊断

- declaration 和 binding 错误使用新的轻量 `CalxProgramError`，并实现
  `diagnostic() -> DiagnosticView`；
- 函数体错误继续使用 `ValidationError`，保留 function、syntax index、span 与
  expected/actual stack；
- 两者 phase 都是 validation，沿用 `CALX_VALIDATION` 类别，message 明确 local/global/import；
- strict declaration 中的 `Dynamic`、`Nil` boundary 或 result/callback arity mismatch 在实例化前
  返回 validation error；
- legacy uninitialized local/global 的首次读取使用 `CALX_RUNTIME_TRAP`，不得悄悄得到 nil；
- guest 经过 Dynamic 后触发的操作数错误仍是 `CALX_RUNTIME_TRAP`；
- callback 或 callback result 违反 host contract 使用 `CALX_HOST_IMPORT` 且无 VM snapshot；
- 不把 global values、locals、callbacks 或完整 VM state 放入 inline validation error。

## 兼容与迁移矩阵

| 旧用法 | 0.3 映射 | 静态保证 | 推荐迁移 |
| --- | --- | --- | --- |
| non-nil function params/results | `Known(CalxType)` | 完整 | 无需迁移 |
| `nil` function boundary | legacy-only | 无 strict guarantee | 用零结果、Option/Result 编码 |
| source `local.new $x` prefix | legacy Dynamic + Uninitialized | 仅 index/arity | 改为 `local $x TYPE` |
| public `CalxSyntax::LocalNew` | legacy Dynamic + Uninitialized | 运行时 | builder/local metadata |
| `CalxVM::new` non-nil global | anonymous legacy `mut Known(actual type)` | 类型已知，无 const | named global declaration |
| `CalxVM::new` nil global | legacy Dynamic | 运行时 | 显式非 nil typed initializer |
| source/public `global.new` | legacy Dynamic + Uninitialized | 运行时 | module global declaration |
| tuple import `(fn, arity)` | `Dynamic × arity -> Dynamic` | 仅 arity | `CalxHostBinding` + `import-fn` |
| void callback returns `Nil` | legacy value result | 运行时 | `CalxHostCallback::Void` |
| void `run()` returns `Nil` | legacy adapter | 不进入 typed VM | `run_typed() -> Void` |
| explicit `const nil` | explicit runtime value | Known Nil on stack | 不用作缺失/未初始化 |
| `parse_function` | legacy function, no module metadata | 原有边界 | `parse_program` |

兼容测试必须直接运行全部现有 demos，并保留至少一个旧 Rust embedding tuple import、一个
legacy source local/global；typed 与 legacy profile 分开测试，禁止在 strict program 中混用。

### 当前仓库兼容审计

RFC 起草时的 in-repo consumer 已逐项核对：

- `demos/named.cirru` 与 `demos/sum.cirru` 的 `local.new` 都位于 function declaration
  prefix，implementation PR 应直接迁移为 concrete typed local，而不是把 Dynamic 带入新路径；
- `demos/sum.cirru` 的 `call-import log2` 没有 source declaration，CLI
  `standard_imports()` 目前返回 `Calx::Nil`；implementation PR 应增加 `(i64 i64 ->)` 声明并
  迁移为 `CalxHostCallback::Void`，旧 tuple 只留一条兼容回归；
- `global.new` 只由语义/opcode 回归覆盖，继续作为 legacy Dynamic 指令测试，并增加
  read-before-set trap，确认不再把 nil 当作 initialized value；
- tests 与 benchmarks 中存在多处 `CalxFunc` struct literal，representation PR 必须先提供
  constructor/builder 并迁移仓库自身用法；
- tuple `CalxImportsDict` 被 CLI、tests 与 benchmarks 广泛使用，0.3 不能直接删除或加
  hard deprecation warning。

## 与 WebAssembly 的对应与有意差异

### 相同点

- local index space 中参数在前、声明 local 在后；local 可赋值但类型固定；
- local 在函数调用建立 frame 时按类型默认初始化；
- global 声明包含 value type、mutability 与 initializer；`global.set` 只允许 mutable；
- import declaration 携带外部类型，宿主提供项必须在实例化前匹配；
- module validation 先收集函数/global/import 类型上下文，再验证函数体。

### 有意差异

- Calx runtime 仍支持显式 `nil/bool/str/list`；strict storage/function/import boundary 排除
  `nil`，legacy profile 才存在 `Dynamic`，Wasm core value types 没有这两种兼容行为；
- Calx global initializer 0.3 只接受 literal，不实现 Wasm constant expression；
- Calx 只导入单名同步 function，暂不使用 Wasm 的 module/item 双层名，也不导入
  global/table/memory/tag；
- Calx host callback 0.3 返回零或一个值，暂不实现 Wasm function type 的多结果；
- legacy `local.new/global.new` 是 Calx 兼容指令，不是 Wasm module declaration；
- Calx 的 truthiness、tail call 和教学指令保持自身语义，不由本 RFC 改成 Wasm 行为。

## 被拒绝的替代方案

### 首次赋值推断 local/global 类型

这需要为 if 合流和 loop 建立数据流不动点，也会让错误位置依赖控制路径。strict VM 采用
显式 concrete declaration；无法声明的旧边界留在 legacy profile，而不是污染 typed path。

### 在 strict declaration 中保留显式 `dynamic`

即使 token 是显式的，它仍会让 local/global 热路径失去稳定类型，并把错误推迟到 runtime。
因此 Dynamic 只用于标识无法立即删除的 legacy contract，不是新模块的常规逃生口。

### 用 `Calx::Nil` 统一表示 void 与未初始化

这会让合法 nil 数据、函数无结果和状态错误无法区分，也迫使 host 日志函数产生无意义的
运行值。strict API 使用 `CalxRunResult::Void`、void callback 和独立 Uninitialized 状态。

### 从 Rust callback function pointer 推断 import signature

Rust function pointer 只暴露 Rust 参数容器和返回类型，无法表达 guest 参数列表中的
`CalxType`。signature 必须由 host binding 显式声明并在 runtime 守卫。

### 让所有旧 API 自动变成 Known

tuple import 只有 arity，`global.new/local.new` 也没有 type token；把它们标成 Known 会制造
虚假静态保证。兼容映射必须保留 Dynamic。

### 一次删除所有动态指令和旧 constructor

这会同时破坏 demos、benchmarks 与 Calcit embedding，超出 0.3 范围。先提供 typed path、
迁移测试和清楚的诊断，再在未来 edition 讨论删除。

## 测试与验收映射

第二、三阶段 PR 至少覆盖：

- typed local 非 nil 默认值、get/set/tee、known mismatch、duplicate/late declaration；
- immutable/mutable global、initializer mismatch、named/numeric index、unknown index；
- typed import 零/单 result、guest mismatch、binding mismatch、callback result violation；
- strict program 对 Dynamic/Nil boundary 的默认拒绝及 zero Dynamic invariant；
- legacy Dynamic local/global/import 在 validation trace 中保持 Dynamic，并在运行时保留检查；
- legacy local/global read-before-set trap，void typed run/callback 不产生 `Calx::Nil`；
- source span 指向 declaration 或失败 call instruction；
- 旧 demos、legacy `CalxVM::new`、tuple imports 和 manual `CalxFunc` migration；
- Debug/Release、严格 Clippy、package verify 全部通过。

完成定义：strict program 的 Dynamic boundary 数量为零，且 local/global/import/function
boundary 不含 Nil；所有 mismatch 在执行前失败，local/global 热路径不需要动态类型守卫。
只有明确进入 legacy profile 或不透明 callback 行为可以把类型错误保留到运行时，且诊断
不得称其为静态安全。

## 未决问题 / Open questions

以下问题推迟到后续 RFC，不阻塞 0.3：

- typed host ABI 何时升级为多结果；
- 是否允许 closure、captured state 或 async callback；
- 是否引入 module/item 双层 import name；
- global import/export、constant expression 和跨模块 linking；
- `link` 获得运行时值后采用何种默认初始化；
- 哪个未来 Calx instruction edition 可以移除 source-level `local.new/global.new`；
- legacy dynamic report 的 machine-readable warning schema；
- `Calx::Nil` runtime value 是否在未来 typed edition 中进一步限制为容器内显式数据。

## 参考

- WebAssembly module syntax and index spaces：
  <https://webassembly.github.io/spec/core/syntax/modules.html>
- WebAssembly module validation：
  <https://webassembly.github.io/spec/core/valid/modules.html>
- WebAssembly validation algorithm：
  <https://webassembly.github.io/spec/core/appendix/algorithm.html>
- RFC 0001：[`0001-validation-and-traps.md`](0001-validation-and-traps.md)
