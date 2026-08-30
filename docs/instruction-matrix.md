# Calx 指令实现与测试矩阵

> 适用版本：0.2.x / 0.3 开发基线  
> 状态：每次增加、删除或改变 opcode 时必须同步更新

本矩阵把 [`instruction-set.md`](instruction-set.md) 的语义说明映射到 parser、validator、lowering、interpreter 和自动测试。它不表示 Calx 与 WebAssembly 二进制兼容。

状态：

- **直接**：该阶段直接处理此 opcode；
- **结构化**：parser 展开或 lowering 生成内部指令；
- **拒绝**：在进入下一阶段前返回明确错误；
- **不适用**：该形式在更早阶段已消失。

测试缩写：

- `matrix/*`：`tests/opcode_matrix_tests.rs`；
- `semantics/*`：`tests/vm_semantics_tests.rs`；
- `validator/*`：`tests/validator_tests.rs`；
- `demos`：`try.sh` 对全部 demo 的稳定输出断言。

## Cirru source opcodes

| Opcode | Parser | Validator | Lowering | Interpreter | 自动测试 | 状态与说明 |
| --- | --- | --- | --- | --- | --- | --- |
| `local.new` | 直接 | 直接，产生 Dynamic local | 直接 | 直接 | `matrix/local-stack` | 支持 |
| `local.get` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack`, `validator/local` | 支持 |
| `local.set` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack`, `validator/local` | 支持 |
| `local.tee` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack`, `semantics/local-tee` | 支持 |
| `global.new` | 直接 | 直接，产生 Dynamic global | 直接 | 直接 | `matrix/local-stack` | 支持 |
| `global.get` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack`, `semantics/global` | 支持 |
| `global.set` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack`, `semantics/global` | 支持 |
| `const` | 直接 | 直接 | 直接 | 直接 | `matrix/*`, `demos` | 支持标量；list literal 拒绝 |
| `dup` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack` | 支持 |
| `drop` | 直接 | 直接 | 直接 | 直接 | `matrix/local-stack` | 支持 |
| `i.add` | 直接 | `i64 i64 -> i64` | 直接 | wrapping | `matrix/local-stack`, `semantics/integer` | 支持 |
| `i.mul` | 直接 | `i64 i64 -> i64` | 直接 | wrapping | `matrix/integer` | 支持 |
| `i.div` | 直接 | `i64 i64 -> i64` | 直接 | 除零/溢出 trap | `matrix/integer`, `semantics/integer` | 支持 |
| `i.rem` | 直接 | `i64 i64 -> i64` | 直接 | 除零 trap | `matrix/integer` | 支持 |
| `i.neg` | 直接 | `i64 -> i64` | 直接 | wrapping | `matrix/integer` | 支持 |
| `i.shr` | 直接 | `i64 i64 -> i64` | 直接 | masked shift | `matrix/integer` | 支持 |
| `i.shl` | 直接 | `i64 i64 -> i64` | 直接 | masked shift | `matrix/integer`, `semantics/integer` | 支持 |
| `i.eq` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer`, `demos` | 支持 |
| `i.ne` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer` | 支持 |
| `i.lt` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer`, `demos` | 支持 |
| `i.le` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer`, `demos` | 支持 |
| `i.gt` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer` | 支持 |
| `i.ge` | 直接 | `i64 i64 -> bool` | 直接 | 直接 | `matrix/integer`, `demos` | 支持 |
| `add` | 直接 | 同型 `i64/f64` 或 Dynamic | 直接 | wrapping i64 / f64 | `matrix/float`, `demos` | 部分静态支持 |
| `mul` | 直接 | 同型 `i64/f64` 或 Dynamic | 直接 | wrapping i64 / f64 | `matrix/float` | 部分静态支持 |
| `div` | 直接 | `f64 f64 -> f64` | 直接 | IEEE 754 | `matrix/float` | 支持 f64 |
| `neg` | 直接 | `f64 -> f64` | 直接 | IEEE 754 | `matrix/float`, `demos` | 支持 f64 |
| `block` | 结构化 | control frame | `Nop` + branch targets | 内部指令 | `matrix/control`, `validator/control`, `demos` | 支持 |
| `loop` | 结构化 | loop control frame | `Nop` + loop target | 内部指令 | `matrix/control`, `demos` | 支持 |
| `br` | 直接 | label types + unreachable | `Branch` | 直接 | `matrix/control`, `semantics/branch` | 支持 |
| `br-if` | 直接 | condition + label types | `BranchIf` | 直接 | `matrix/control`, `semantics/branch` | 支持 |
| `if` | 结构化 | if control frame | `JmpIf` / `Jmp` | 直接 | `matrix/control`, `semantics/truthiness`, `demos` | 支持 |
| `do` | 结构化 branch body | 不适用 | 不适用 | 不适用 | `matrix/control`, `parser/malformed` | 只允许作为 `if` branch wrapper |
| `call` | 直接 | 函数签名 | indexed `Call` | 直接 | `matrix/control`, `validator/call`, `demos` | 支持 |
| `return-call` | 直接 | 参数与结果签名 | indexed `ReturnCall` | 直接 | `matrix/control`, `semantics/return` | 支持 |
| `call-import` | 直接 | arity + Dynamic result | 直接 | host callback | `matrix/control`, `demos` | 部分静态支持 |
| `return` | 直接 | function result types | 直接 | frame return | `matrix/*`, `semantics/return` | 支持 |
| `unreachable` | 直接 | unreachable stack | 直接 | trap | `semantics/traps`, `validator/unreachable` | 支持 |
| `nop` | 直接 | 直接 | 直接 | 直接 | `matrix/diagnostic` | 支持 |
| `quit` | 直接 | unreachable stack | 直接 | trap，不退出宿主 | `semantics/traps` | 支持 |
| `echo` | 直接 | pop Dynamic | 直接 | stdout | `matrix/diagnostic`, `demos` | 教学扩展 |
| `assert` | 直接 | pop Dynamic | 直接 | false 时 trap | `matrix/diagnostic`, `demos` | 教学扩展 |
| `inspect` | 直接 | 无栈变化 | 直接 | stdout | `matrix/diagnostic`, `demos` | 教学扩展 |
| `;;` | parser 丢弃 | 不适用 | 不适用 | 不适用 | `demos` | Cirru source comment marker |
| `new-list` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `list.get` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `list.set` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `new-link` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `and` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `or` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |
| `not` | 拒绝 | 防御性拒绝 | 不适用 | 防御性错误 | `semantics/reserved` | 保留，未定义 |

## Internal markers and instructions

| Internal form | Producer | Consumer | Test evidence | Notes |
| --- | --- | --- | --- | --- |
| `BlockEnd` | parser | validator/lowering | `matrix/control`, `validator/control` | 不开放为 source opcode |
| `ElseEnd`, `ThenEnd` | parser | validator/lowering | `matrix/control`, `demos/if` | 不开放为 source opcode |
| `Jmp`, `JmpIf` | lowering | interpreter | `matrix/control`, `demos/if` | if lowering |
| `Branch`, `BranchIf` | lowering | interpreter | `matrix/control`, `semantics/branch` | 携带 target base/arity |
| `JmpOffset`, `JmpOffsetIf` | 旧的公开 IR variant | interpreter | `semantics/malformed-public-instructions` | 当前 source/lowering 不生成；非法负 target 返回错误 |

## Public-input safety boundary

以下行为属于 CI 回归范围：

- 缺失 block/loop 类型签名、空 `do` 和其他 malformed Cirru 返回 parser error，不 panic；
- validator 在 lowering 前拒绝栈、类型、label、local/global 和函数签名错误；
- guest 的 `unreachable`、`quit`、算术 trap 和 assert failure 返回错误，不 panic 或退出宿主；
- 即使 embedding consumer 手工构造非法 function index 或 offset instruction，公开 VM API 也返回 `CalxError`；
- `process::exit` 只允许用于 CLI 参数 parser 的正常 help/error 退出，不可由 guest instruction 到达；
- 静态 regex 的构造属于启动期宿主不变量，不接受 guest pattern、索引或控制数据。

新增 opcode 时，PR 必须同时更新本矩阵、`instruction-set.md`、validator、interpreter，以及至少一条执行测试或明确拒绝测试。
