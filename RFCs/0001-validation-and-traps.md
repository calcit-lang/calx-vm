# RFC 0001：类型验证与 Trap 边界

- 状态：已实现第一阶段，保留后续扩展
- 目标版本：0.3
- 关联 Issue：#24

## 摘要

Calx 在 lowering 前增加独立的单遍验证器。验证器同时维护类型化操作数栈和控制栈，在解释器运行前检查数值操作、local/global、函数签名、返回、block/loop/if 和 branch label。

现有 Calcit 风格动态边界使用显式的 `Dynamic` 验证类型。`Dynamic` 表示“静态信息不足，必须保留运行时检查”，不表示任意值已经通过静态类型证明。

验证错误与运行时 trap 分属不同阶段：

```text
Cirru
  -> parse error
CalxSyntax
  -> ValidationError
validated syntax
  -> lowering
CalxInstr
  -> CalxError / trap
```

## 动机

原有 `preprocess` 同时承担跳转 lowering 和栈高度检查，只能发现操作数数量问题，不能区分 `i64`、`f64`、`bool` 等值。其结果是：

- 错误数值类型可能直到 interpreter 才失败；
- 函数参数、返回值和 block result 只有数量约束；
- `return`、`br`、`unreachable` 后的死代码缺少栈多态；
- lowering 与验证互相耦合，难以实现 `calx check/explain`；
- 同一语义可能被 validator 和 lowering 以不同规则重复判断。

WebAssembly 将验证定义为抽象语法上的类型系统，并以操作数栈和控制帧给出单遍算法。Calx 采用这一结构，但保留动态值、truthiness、宿主 import 和不以 `if` 作为 branch label 等项目差异。

## 目标与非目标

本阶段目标：

- 验证发生在任何 lowering 修改之前；
- 已知类型的错误在执行前被拒绝；
- 结构化控制流使用 label 参数/结果类型，而非仅比较栈高度；
- 不可达代码使用栈多态，避免伪 underflow；
- 错误包含函数、syntax index、原因和当时的操作数类型栈；
- lowering 只生成运行指令和跳转，不再重复决定语义是否合法。

本阶段不解决：

- Cirru source span；当前定位单位仍是扁平 `CalxSyntax` index；
- 稳定错误码和完整 diagnostic renderer；
- typed local declaration、typed global declaration 和 typed import ABI；
- 多返回值如何映射到当前只返回一个 `Calx` 的 `run` API；
- binary container、线性内存、table、GC 或 JIT。

## 验证类型

```text
ValidationType := Known(CalxType) | Dynamic
```

匹配规则：

- `Known(T)` 接受 `Known(T)`；
- 不同的两个 `Known` 类型不匹配；
- `Dynamic` 与任意期望类型匹配，但对应操作仍须保留运行时检查；
- 已知数值操作产生已知结果，例如两个 `i64` 经 `i.add` 产生 `Known(I64)`；
- 重载 `add/mul` 遇到 `Dynamic` 时产生 `Dynamic`，不能假定运行时一定是整数或浮点。

`Dynamic` 当前来自：

- 无类型标注的 `local.new`；
- `global.new`；
- 仅声明 arity、没有参数/返回类型的 `call-import` 返回值。

函数参数、常量和初始 globals 具有 `Known` 类型。

## 操作数栈

验证器按执行顺序维护 `Vec<ValidationType>`，栈尾为顶部。每条指令声明 pop/push 规则：

| 指令类别 | 输入 | 输出 |
| --- | --- | --- |
| `const T` | `[]` | `[T]` |
| `local.get x` | `[]` | `[local[x]]` |
| `local.set x` | `[local[x]]` | `[]` |
| `local.tee x` | `[local[x]]` | `[local[x]]` |
| `i.add/mul/div/rem/shl/shr` | `[I64 I64]` | `[I64]` |
| `i.eq/ne/lt/le/gt/ge` | `[I64 I64]` | `[Bool]` |
| `div` | `[F64 F64]` | `[F64]` |
| `call f` | `params(f)` | `results(f)` |
| `call-import f` | `Dynamic × arity(f)` | `[Dynamic]` |
| `echo/assert/drop` | `[Dynamic]` | `[]` |

Calx 当前允许任意值参与 truthiness，因此 `if`、`br-if` 和 `assert` 只要求一个操作数，不把条件强制为 Wasm 的 `i32`。这是明确的 Calx 扩展。

## 控制帧

每个 function、block、loop 和 if 建立一个控制帧：

```text
ControlFrame {
  kind,
  height,
  start_types,
  end_types,
  unreachable,
  first_branch_unreachable,
}
```

- `height` 是进入结构时、移除参数后的外层栈高度；
- `start_types` 是结构参数；
- `end_types` 是正常结束结果；
- loop label 接收 `start_types`；
- block label 接收 `end_types`；
- Calx 的 `br depth` 只对 block/loop 计数，跳过 if frame；
- if 的两条分支分别验证，并在第二条分支开始前恢复入口栈；
- 两条 if 分支都不可达时，把不可达状态合并回外层帧。

Calx parser 当前把 else 分支排在扁平 syntax 的前半段，以便 lowering 生成条件跳转。验证器按实际扁平顺序处理，但这只是过渡表示；未来结构化 AST 不应预先携带绝对 jump index。

## 不可达栈多态

执行 `return`、无条件 `br`、`unreachable`、`quit` 或 `return-call` 后：

1. 操作数栈截断到当前控制帧的 `height`；
2. 当前帧标记为 `unreachable`；
3. 当后续死代码试图从该高度继续 pop 时，验证器提供 `Dynamic` bottom 值；
4. 一旦死代码显式 push 了具体类型，后续指令仍检查这些具体类型，不能用不可达状态掩盖真实的内部类型矛盾。

这允许验证用于演示 Wasm 的 polymorphic stack，同时保持错误报告可理解。

## Local、Global 与 Import 的保证边界

### Local

函数参数 local 使用签名中的固定类型。`local.new` 目前没有类型参数，因此产生 `Dynamic` local。对固定参数执行 `local.set/tee` 必须类型匹配；动态 local 保留 interpreter 检查。

后续建议将语法扩展为显式 typed local declaration，而不是依赖首次赋值推断。首次赋值推断需要处理分支合并和循环不动点，不适合作为教学实现的默认复杂度。

### Global

传入 VM 的初始 global 根据实际 `Calx` 值获得已知类型。运行时 `global.new` 产生 `Dynamic`。在函数体内可以检查已知分配顺序，但跨函数动态创建 global 不是可靠的模块声明机制，后续应迁移到 program/module metadata。

### Import

现有 import 字典只记录 Rust function pointer 和 arity。验证器检查参数数量，参数和单个返回值均视为 `Dynamic`。typed host ABI 应通过单独 RFC 增加完整签名，而不是从 Rust function pointer 猜测。

## 错误与 Trap

`ValidationError` 当前公开：

- `function`；
- `instruction_index`；
- `message`；
- `operand_stack`。

它不包含 VM snapshot，因为验证尚未执行程序。运行期 trap 使用轻量 `CalxError`，并通过可选 boxed `CalxErrorSnapshot` 按需保留 stack、frame 和 globals；宿主 `new_raw` 错误没有 VM snapshot。该布局由 #21 完成，避免把大型快照放在每个 `Result` 的 inline `Err` payload 中。

后续 diagnostic 层应增加稳定错误码和 source span，但不得把 parse、validation 和 runtime trap 重新合并成一个含糊字符串。

## Lowering

`CalxVM::preprocess` 先调用 `validate_program`。验证成功后，lowering 不再重复返回栈类型/高度错误，只维护生成 block targets 和内部 jump 所需的近似高度。

这一顺序保证验证失败时不会产生部分 `CalxInstr`，也避免旧高度检查与不可达栈多态冲突。

源级 `br/br-if` lowering 为内部 `Branch/BranchIf { target, base, arity }`。执行 branch 时，VM 把顶部 `arity` 个 label 结果暂存，将栈截断到目标控制帧 base，再放回结果。`return` 和 `return-call` 同样丢弃调用帧内不属于结果/参数的中间值。这保证 validator 允许的结构化栈清理与 runtime 一致。

## 被拒绝的替代方案

### 只扩展 `CalxInstr::stack_arity`

单个 `(usize, usize)` 无法表达具体类型、重载操作、函数签名、label types 或不可达状态，继续扩展只会把验证逻辑散落在 lowering 中。

### 把所有 Calx 值视为 Dynamic

这只能把 runtime error 改名为 validation success，不能达到教学或编译前检查目的。已知常量、函数签名和数值操作必须保持精确类型。

### 立即强制所有 local/global/import 静态类型化

这会一次性破坏现有 demos、嵌入 API 和 Calcit 动态边界。第一阶段用 `Dynamic` 明确隔离缺口，再分别设计 typed declarations 和 host ABI。

## 测试要求

- 每类已知类型规则至少包含一个成功和失败测试；
- 错误测试检查函数名、syntax index 和期望/实际类型；
- function call、return、block result 和 branch label 必须覆盖；
- `return/br/unreachable` 后的多态 underflow 必须覆盖；
- 所有原有 demos 必须先通过 validator，再通过 lowering 和 interpreter；
- 新 validator 不得给 interpreter 热循环增加逐指令运行期开销。

## 参考

- WebAssembly Validation Conventions：<https://webassembly.github.io/spec/core/valid/conventions.html>
- WebAssembly Validation Algorithm：<https://webassembly.github.io/spec/core/appendix/algorithm.html>
- WebAssembly Instruction Validation：<https://webassembly.github.io/spec/core/valid/instructions.html>
