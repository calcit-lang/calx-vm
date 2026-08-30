# RFC 0002：类型化 Local、Global 与 Import 边界 / Typed Local, Global, and Import Boundaries

- 状态：提议 / Proposed
- 目标版本：0.3
- 关联 Issue：#31
- 前置：RFC 0001、#30 source-aware diagnostics

## 摘要 / Summary

Calx 将增加显式 `CalxProgram` 模块元数据，用于声明函数内 local、模块 global 与宿主
function import 的类型契约。已声明边界使用 `Known(CalxType)` 在执行前验证；仍需动态接入
Calcit 或旧 embedding API 时，必须显式使用 `Dynamic`，并保留运行时检查。

The proposal adds explicit `CalxProgram` metadata for function locals, module
globals, and host function imports. Declared `Known(CalxType)` boundaries are
validated before execution. Existing Calcit and embedding boundaries remain
available through an explicit, observable `Dynamic` escape hatch with runtime
checks.

本 RFC 只固定设计。实现按不超过三个 PR 拆分：

1. RFC 与文档索引；
2. program representation、Rust API 与 parser；
3. validator、runtime、CLI 与兼容回归。

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
- `Dynamic` 在 Cirru、Rust metadata、`check`/`explain` 中都可观察；
- 旧 demos 与 `CalxVM::new(fns, globals, imports)` 保持兼容并有迁移路径。

## 非目标

- 不增加 memory、table、GC、持久化集合或 global import；
- 不设计 async/closure host callback；
- 不在 0.3 支持 host import 的零/多返回值；
- 不实现 Calcit → Calx 编译器；
- 不删除 public `CalxSyntax::LocalNew/GlobalNew` 或旧 import tuple；
- 不把 declaration、validation、runtime 和 host error 合并为一个大型错误对象。

## 类型契约

模块边界使用独立于 validator 内部栈状态的公开类型：

```rust
pub enum CalxBoundaryType {
  Known(CalxType),
  Dynamic,
}
```

规则：

- `Known(T)` 只接受运行值或静态栈值 `T`；
- `Dynamic` 表示静态信息不足，不表示任意值已经被证明正确；
- `Known` 与 `Dynamic` 的匹配沿用 RFC 0001：验证可以继续，但消费者保留运行时检查；
- `dynamic` 是 Cirru 中的显式类型 token，不加入 `CalxType`，避免把“未知”伪装成运行值类型；
- `ValidationType` 在 validator 内部继续表示操作数栈状态；实现阶段可与
  `CalxBoundaryType` 相互转换，但两者职责不同。

## Cirru 源码语法

### Typed local declaration

local declaration 位于函数签名之后、第一条可执行指令之前，不生成 `CalxSyntax`：

```cirru
fn accumulate (($input i64) -> i64)
  local $sum i64
  local $scratch dynamic
  local.get $input
  local.set $sum
  local.get $sum
  return
```

约束：

- 形式固定为 `local $name TYPE`；name 必须唯一；
- 参数 index 在前，声明 local 按源码顺序在后，与 Wasm local index space 一致；
- 所有 local 均可通过 `local.set/tee` 修改，类型本身不可改变；
- known local 在每次函数调用建 frame 时使用类型默认值初始化：`nil → nil`、
  `bool → false`、`i64 → 0`、`f64 → 0.0`、`str → ""`、`list → []`；
- `dynamic` 默认值为 `nil`；`link` 尚无运行值，0.3 拒绝其 local/global declaration；
- 声明不得出现在 block/loop/if 内，也不得出现在可执行指令之后。

旧源码 `local.new` 只在函数 declaration prefix 中兼容：

```cirru
fn legacy ()
  local.new $value
  ;; equivalent to: local $value dynamic
```

无 name 的 `local.new` 使用下一个数字 local name。parser 将 source-level legacy form
规范化为 function-entry dynamic declaration，不再让控制流决定 local layout。出现在执行区
或嵌套控制结构中的 `local.new` 将产生 parse diagnostic。手工构造的
`CalxSyntax::LocalNew` 仍按旧动态指令执行，属于 Rust IR compatibility escape hatch。

### Typed global declaration

global 是 top-level module declaration：

```cirru
global $counter (mut i64) 0
global $build-id (const str) "|dev build"
global $legacy-slot (mut dynamic) nil

fn main (-> i64)
  global.get $counter
  return
```

约束：

- 形式固定为 `global $name (MUTABILITY TYPE) INITIALIZER`；
- mutability 只接受 `const` 或 `mut`；
- initializer 使用当前 `const` 标量 literal 规则，`Known(T)` 必须与 initializer 类型一致；
- `Dynamic` 接受任意 initializer，但 `global.get` 产生 `Dynamic`；
- global index 按 declaration 出现顺序固定；`global.get/set` 同时接受 `$name` 和旧数字 index；
- `global.set` 对 `const` global 总是在验证阶段失败；对 `mut Known(T)` 检查 `T`；
- declarations 可与 functions 交错书写，但 parser 先做 module pre-pass，因此所有函数看到
  同一个完整 global context；duplicate name 在 parse/module validation 阶段失败。

旧 `CalxVM::new` 传入的 globals 映射为匿名、`mut Known(actual.value_type())` 声明。旧
`global.new` 继续追加匿名 `mut Dynamic` slot，只作为兼容逃生口；它不能建立可供其他函数
静态引用的 module declaration。新源码和 builder 不应依赖跨函数的 `global.new` 顺序。

### Typed function import declaration

0.3 只定义单名、同步、单结果 function import：

```cirru
import-fn add2 (i64 i64 -> i64)
import-fn log2 (dynamic dynamic -> dynamic)

fn main (-> i64)
  const 20
  const 22
  call-import add2
  return
```

约束：

- 形式固定为 `import-fn NAME (PARAMS... -> RESULT)`；0.3 必须恰有一个 result；
- name 在 Calx module 中唯一，并继续由 `call-import NAME` 引用；
- validator 按逆栈顺序消费 params，随后压入 result；
- `Known` 参数 mismatch 在执行前失败，`Dynamic` 参数只证明 arity；
- source declaration 与 host binding 宣告的 signature 必须在 VM 实例化、执行任何 guest
  指令之前完全一致；
- callback 是不透明宿主代码。即使 binding 宣告 `Known(T)`，VM 仍检查 callback 返回值，
  违反声明时返回 host boundary error，而不是把错误归因于 guest validator；
- 旧源码可省略 import declaration。旧 tuple `(callback, arity)` 会被提升为
  `Dynamic × arity -> Dynamic`，因此现有 demos 保持运行但不声称静态安全。

## Rust 表示与 API

第二阶段实现采用以下逻辑模型；字段可以使用 `Rc<Vec<_>>` 等现有共享布局，但语义不得改变：

```rust
pub struct CalxLocalDecl {
  pub name: Rc<str>,
  pub value_type: CalxBoundaryType,
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
  pub result: CalxBoundaryType,
  pub span: Option<SourceSpan>,
}

pub struct CalxProgram {
  pub functions: Vec<CalxFunc>,
  pub globals: Vec<CalxGlobalDecl>,
  pub imports: Vec<CalxImportDecl>,
}
```

`CalxFunc` 增加非参数 `locals: Rc<Vec<CalxLocalDecl>>`。`local_names` 暂时保留以兼容
debug/display consumer；实现应提供 constructor/builder，避免继续要求下游手写不断增长的
struct literal。

宿主绑定与 guest declaration 分离：

```rust
pub type CalxHostFn = fn(&[Calx]) -> Result<Calx, CalxError>;

pub struct CalxHostBinding {
  pub callback: CalxHostFn,
  pub params: Rc<Vec<CalxBoundaryType>>,
  pub result: CalxBoundaryType,
}

pub type CalxHostBindings = HashMap<Rc<str>, CalxHostBinding>;
```

新入口：

```rust
CalxVM::from_program(program: CalxProgram, bindings: CalxHostBindings)
  -> Result<CalxVM, CalxProgramError>
```

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
`CalxProgram`。AST-only `parse_function` 继续返回 legacy function，source spans 与 module
declarations 仍不可用。

## Module validation 与实例化

处理顺序固定为：

```text
Cirru declarations + functions
  -> parse/module pre-pass
CalxProgram
  -> declaration validation
  -> function validation
validated CalxProgram + host bindings
  -> binding signature validation
  -> globals/frame instantiation
CalxInstr
  -> runtime + dynamic/host guards
```

### Declaration validation

在函数验证前完成：

- local/global/import name 唯一性和 type token 合法性；
- known global initializer 类型；
- import 恰有一个 result；
- `link` declaration 拒绝；
- named/numeric global index 可解析；
- legacy declaration 显式标记为 Dynamic。

### Function validation

validator context 改为：

```text
locals  = Known(function params) ++ function local declarations
globals = module global contracts ++ legacy runtime globals
imports = module import signatures ++ synthesized legacy signatures
```

规则：

- `local.get/global.get` 压入对应 contract；
- `local.set/tee` 与 mutable `global.set` 使用 RFC 0001 matching rule；
- immutable `global.set` 在 pop 之前失败，保证 error stack 不被部分修改；
- typed `call-import` 精确消费参数并压入 result；
- unknown index/name、duplicate declaration、known mismatch 均在 lowering 前失败；
- 经过 Dynamic 的操作在 trace 中保持 Dynamic，不得升级为 Known。

### Host binding validation 与 runtime guards

`CalxVM::from_program` 在创建可执行 VM 前检查：

- 每个 declared import 恰有一个 binding；
- params/result contract 与 declaration 完全相等；
- host 可以提供 module 未使用的额外 bindings，它们不进入 guest import context；
- strict `from_program` 不从 bindings 猜测缺失的 source declaration，undeclared
  `call-import` 在 module validation 阶段失败；
- 只有旧 `validate_program` / `CalxVM::new` compatibility path 才从
  `CalxImportsDict` 的 arity 合成 Dynamic contracts。CLI 的未迁移 demos 暂走该路径。

callback 执行前仍检查实参数量和所有 `Known` 参数的运行值类型，返回后检查
`Known(result)`。这是宿主信任边界，不因 guest validation 成功而删除。callback 自己返回的
`CalxError::new_raw` 及 signature violation 都属于 host phase，不伪造 VM snapshot。

## Dynamic 的可观察性

Dynamic 不是 warning-free 的静态成功。实现必须同时提供：

- Rust metadata 中显式的 `CalxBoundaryType::Dynamic`；
- `trace_validation` / `calx explain` 中现有的 `Dynamic` 栈显示；
- `calx check` 成功摘要中的 dynamic boundary 数量；
- source-aware explain 输出中每个 dynamic local/global/import declaration 的位置；
- runtime mismatch 继续使用实际指令 source span 或 host boundary message。

`calx check` 遇到 Dynamic 仍返回成功；0.3 不增加 `--deny-dynamic`。这保证动态 Calcit
边界可用，同时避免把它描述为完整静态证明。

## 错误与诊断

- declaration 和 binding 错误使用新的轻量 `CalxProgramError`，并实现
  `diagnostic() -> DiagnosticView`；
- 函数体错误继续使用 `ValidationError`，保留 function、syntax index、span 与
  expected/actual stack；
- 两者 phase 都是 validation，沿用 `CALX_VALIDATION` 类别，message 明确 local/global/import；
- guest 经过 Dynamic 后触发的操作数错误仍是 `CALX_RUNTIME_TRAP`；
- callback 或 callback result 违反 host contract 使用 `CALX_HOST_IMPORT` 且无 VM snapshot；
- 不把 global values、locals、callbacks 或完整 VM state 放入 inline validation error。

## 兼容与迁移矩阵

| 旧用法 | 0.3 映射 | 静态保证 | 推荐迁移 |
| --- | --- | --- | --- |
| function params | `Known(CalxType)` | 完整 | 无需迁移 |
| source `local.new $x` prefix | function-entry `local $x dynamic` | 仅 index/arity | 改为 `local $x TYPE` |
| public `CalxSyntax::LocalNew` | runtime Dynamic local | 运行时 | builder/local metadata |
| `CalxVM::new` initial global | anonymous `mut Known(actual type)` | 类型已知，无 const | named global declaration |
| source/public `global.new` | anonymous `mut Dynamic` | 运行时 | module global declaration |
| tuple import `(fn, arity)` | `Dynamic × arity -> Dynamic` | 仅 arity | `CalxHostBinding` + `import-fn` |
| `parse_function` | legacy function, no module metadata | 原有边界 | `parse_program` |

兼容测试必须直接运行全部现有 demos，并保留至少一个旧 Rust embedding tuple import、一个
legacy source local/global，以及 typed/dynamic 混用案例。

### 当前仓库兼容审计

RFC 起草时的 in-repo consumer 已逐项核对：

- `demos/named.cirru` 与 `demos/sum.cirru` 的 `local.new` 都位于 function declaration
  prefix，可无行为变化地规范化为 Dynamic local declaration；
- `demos/sum.cirru` 的 `call-import log2` 没有 source declaration，CLI
  `standard_imports()` 必须在 legacy path 合成 `Dynamic Dynamic -> Dynamic`；
- `global.new` 只由语义/opcode 回归覆盖，继续作为 legacy Dynamic 指令测试；
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

- Calx 支持 `nil/bool/str/list` 和显式 `Dynamic`，Wasm core value types 没有该动态逃生口；
- Calx global initializer 0.3 只接受 literal，不实现 Wasm constant expression；
- Calx 只导入单名同步 function，暂不使用 Wasm 的 module/item 双层名，也不导入
  global/table/memory/tag；
- Calx host callback 0.3 恰好返回一个 `Calx`，暂不实现 Wasm function type 的零/多结果；
- legacy `local.new/global.new` 是 Calx 兼容指令，不是 Wasm module declaration；
- Calx 的 truthiness、tail call 和教学指令保持自身语义，不由本 RFC 改成 Wasm 行为。

## 被拒绝的替代方案

### 首次赋值推断 local/global 类型

这需要为 if 合流和 loop 建立数据流不动点，也会让错误位置依赖控制路径。教学 VM 采用显式
declaration，Dynamic 作为明确逃生口。

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

- typed local 默认值、get/set/tee、known mismatch、duplicate/late declaration；
- immutable/mutable global、initializer mismatch、named/numeric index、unknown index；
- typed import 参数/result、guest mismatch、binding mismatch、callback result violation；
- Dynamic local/global/import 在 validation trace 中保持 Dynamic，并在运行时保留检查；
- source span 指向 declaration 或失败 call instruction；
- 旧 demos、legacy `CalxVM::new`、tuple imports 和 manual `CalxFunc` migration；
- Debug/Release、严格 Clippy、package verify 全部通过。

完成定义：所有已声明的 known local/global/import mismatch 在执行前失败；只有显式 Dynamic
或不透明 callback 行为可以把类型错误保留到运行时，且诊断不得称其为静态安全。

## 未决问题 / Open questions

以下问题推迟到后续 RFC，不阻塞 0.3：

- typed host ABI 何时升级为 `Vec<Calx>` 零/多结果；
- 是否允许 closure、captured state 或 async callback；
- 是否引入 module/item 双层 import name；
- global import/export、constant expression 和跨模块 linking；
- `link` 获得运行时值后采用何种默认初始化；
- 哪个未来 Calx instruction edition 可以移除 source-level `local.new/global.new`；
- 是否增加 `calx check --deny-dynamic` 或 machine-readable warning schema。

## 参考

- WebAssembly module syntax and index spaces：
  <https://webassembly.github.io/spec/core/syntax/modules.html>
- WebAssembly module validation：
  <https://webassembly.github.io/spec/core/valid/modules.html>
- WebAssembly validation algorithm：
  <https://webassembly.github.io/spec/core/appendix/algorithm.html>
- RFC 0001：[`0001-validation-and-traps.md`](0001-validation-and-traps.md)
