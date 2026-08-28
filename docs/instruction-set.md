# Calx 指令语义与实现状态

> 适用版本：0.2.x 开发分支  
> 状态：实验性语义基线

本文记录当前可由 Cirru 源码使用的 Calx 指令、运行语义及与 WebAssembly 的主要差异。它是测试和后续验证器的输入，不承诺二进制兼容或长期 API 稳定。

状态定义：

- **支持**：parser、lowering、interpreter 均有实现，并纳入自动测试；
- **部分支持**：可以执行，但静态类型验证或完整边界测试尚未完成；
- **内部**：仅由 lowering 生成，不属于 Cirru 源码接口；
- **保留**：名字或 IR variant 已预留，parser 明确拒绝，interpreter 也会返回错误。

## 值与真假规则

当前值类型为 `nil`、`bool`、`i64`、`f64`、`str`、`list`。`link` 只有类型和指令占位，没有运行时值。

控制条件和 `assert` 统一调用 `Calx::truthy`：

| 值 | 结果 |
| --- | --- |
| `nil`、`false`、整数 `0`、浮点 `0.0` | false |
| `true`、非零整数、非零浮点、字符串、列表 | true |

这是 Calx/Calcit 风格扩展，不是 WebAssembly 条件语义。后续 typed validator 必须显式决定允许哪些条件类型，不能依赖隐式的 Rust 类型分支。

## 指令矩阵

| 类别 | Cirru 指令 | 状态 | 当前语义与限制 |
| --- | --- | --- | --- |
| 常量 | `const` | 支持 | 支持标量常量；不支持 list literal |
| 栈 | `dup`, `drop` | 支持 | 栈下溢返回 VM 错误 |
| local | `local.new/get/set/tee` | 支持 | `local.new` 初始化为 `nil`；索引越界返回错误 |
| global | `global.new/get/set` | 支持 | global 当前没有静态类型和 mutability 声明 |
| 整数 | `i.add`, `i.mul`, `i.neg` | 支持 | `i64` 二补码 wrapping 语义 |
| 整数 | `i.div`, `i.rem` | 支持 | 除零 trap；`i.div` 的 `MIN / -1` trap |
| 整数 | `i.shl`, `i.shr` | 支持 | shift count 按 64 取模；`i.shr` 是有符号右移 |
| 整数比较 | `i.eq/ne/lt/le/gt/ge` | 支持 | 两个 `i64`，产生 `bool` |
| 重载数值 | `add`, `mul` | 部分支持 | 同类型 `i64` 或 `f64`；整数采用 wrapping 语义 |
| 浮点 | `div`, `neg` | 部分支持 | 仅 `f64`；沿用 IEEE 754/Rust 基础运算结果 |
| 结构化控制 | `block`, `loop`, `if`, `br`, `br-if` | 支持 | typed operand/control stack；label 参数/结果与不可达栈多态在 lowering 前验证 |
| 函数 | `call`, `return-call`, `return` | 支持 | 参数与返回类型在 lowering 前验证；动态 local/import 仍可能保留 runtime 检查 |
| 宿主 | `call-import` | 部分支持 | import 只声明参数数量，固定返回一个 `Calx` 值 |
| trap | `unreachable` | 支持 | 返回 VM trap，不触发 Rust panic |
| 宿主安全 | `quit` | 支持 | 返回 VM trap，不允许 guest 直接终止宿主进程 |
| 诊断 | `assert`, `echo`, `inspect` | 支持 | 教学/调试扩展，不对应 Wasm core 指令 |
| 空操作 | `nop` | 支持 | 无状态变化 |
| lowered control | `Jmp*`, `Branch*` | 内部 | lowering 产物；`Branch*` 携带目标栈 base/arity 并清理中间值，不开放为 Cirru 指令 |
| 容器 | `new-list`, `list.get`, `list.set` | 保留 | list mutation/ownership 语义未定义 |
| link | `new-link` | 保留 | 运行时值尚不存在 |
| 布尔 | `and`, `or`, `not` | 保留 | 操作数类型和短路语义尚未定义 |

## 整数语义

整数算术优先采用 WebAssembly 风格的确定性结果：

- `i.add`、`i.mul`、`i.neg` 在 64 bit 范围内 wrapping；
- shift count 只使用低 6 bit；
- `i.div` 在除零和有符号溢出时 trap；
- `i.rem` 在除零时 trap，`i64::MIN % -1` 为 `0`。

Debug 与 Release 构建必须产生相同结果，不能依赖 Rust profile 的 overflow-check 设置。

## 错误边界

Cirru/guest 程序不得触发以下宿主行为：

- Rust `panic!`、`todo!` 或 `unreachable!`；
- 未检查的数组/栈索引；
- `std::process::exit`；
- Debug/Release 不一致的算术溢出。

运行期 `CalxError` 是轻量 message 与可选 boxed `CalxErrorSnapshot`：VM 内 trap 按需保留 stack/frame/globals，宿主 `new_raw` 错误不伪造 VM 状态。parse、validation 和 runtime error 已分阶段返回；稳定错误码、trap kind 与 source span 仍待后续设计。详见 [`diagnostics.md`](diagnostics.md)。

当前 `ValidationError` 已与运行期 `CalxError` 分离，定位到 function 和扁平 syntax index。验证算法及 `Dynamic` 的保证边界见 [`RFC 0001`](../RFCs/0001-validation-and-traps.md)。

## 暂不开放的接口

未设计完成的 binary container 不再出现在 CLI 参数中。重新开放前必须先定义 magic、edition、长度校验、兼容策略及 round-trip/损坏输入测试。

## 参考对应

- WebAssembly 指令索引：<https://webassembly.github.io/spec/core/appendix/index-instructions.html>
- WebAssembly 数值执行规则：<https://webassembly.github.io/spec/core/exec/numerics.html>
- WebAssembly 验证算法：<https://webassembly.github.io/spec/core/appendix/algorithm.html>

Calx 指令名没有带 `i64` 前缀，且包含动态值和教学指令；因此只能逐条声明语义交集，不能仅凭相似命名声称兼容。
