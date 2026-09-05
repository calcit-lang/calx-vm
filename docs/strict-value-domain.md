# Strict value domain / strict 值域

Status: implemented on the development branch, unreleased after 0.4.0.
Tracking: [#59](https://github.com/calcit-lang/calx-vm/issues/59),
[#61](https://github.com/calcit-lang/calx-vm/issues/61).
Source of truth: `src/program.rs`, `src/builder.rs`, `src/validator.rs`,
`tests/strict_value_domain_tests.rs`, and `tests/f64_buffer_tests.rs`.

## 中文

`ValidatedProgram` 的 strict 保证覆盖整个可执行值域：函数参数/结果、local、global、import、
host binding、常量以及 block/loop/if 签名。所有这些位置只接受 `Bool`、`I64`、`F64`、`Str` 和
`F64Buffer`。F64Buffer 继续遵守 RFC 0004 的附加限制：不能作为 global 或 constant。

`List` 只有外层类型标签，没有元素类型合同。空 List 或碰巧同质的 List 也不能成为静态证明，
所以 strict 直接拒绝所有 List，而不递归扫描元素。`Nil`、`Link` 与 Dynamic 同样不准入。
不可达代码仍须满足值域限制；validator 的 unreachable 栈多态只是控制流证明的一部分，
不表示允许动态数据进入程序。

parser 可继续读取 legacy 数据；转为 `CalxProgram` 时检查 strict 值域，错误保留函数和可用的
source span，诊断类别为 `CALX_VALIDATION`。直接构造 Rust IR 经相同门禁。ProgramBuilder
使用同一规则，常量错误在 emission 前返回 `CALX_PROGRAM_BUILD`，不会留下半条指令。
其中 CalxProgram 检查常量的类型准入；F64Buffer 是准入类型，但其 constant 形式仍由后续
`ValidatedProgram` 的 validator 拒绝（builder 会提前拒绝）。F64Buffer global 则由
CalxProgram 的 global 检查与 builder 拒绝。类型准入和特定指令限制是不同阶段。
这没有增加 runtime 类型分支，也没有移除 host 返回值、入口参数或 buffer bounds/conversion 守卫。

从 0.4.0 迁移：

- 删除无意义的 `const nil; drop`，void 使用零结果签名与 `CalxRunResult::Void`。
- 未初始化槽继续使用独立 `Uninitialized` 状态；读取未赋值槽仍 trap。
- 数值批处理在宿主显式构造 F64Buffer，并按 RFC 0004 使用 typed entry/import；没有隐式 List 转换。
- 需要异质数据的现有程序显式保留在 `CalxVM::new` legacy 路径，或在宿主先验证/转换再进入 strict。
- Calcit 现有 scalar/F64Buffer lowering 已限定此子集；本次不修改其 ABI edition 或依赖版本。

CLI 当前按 module declarations 选择执行模式；没有声明的旧程序仍可能进入 legacy。该兼容行为
不会把 legacy 验证当作 strict 证明。编译器直接使用
`ProgramBuilder → CalxProgram → ValidatedProgram → run_typed`。

这一步提供后续优化的前提，不声称已经移除 Rust `Calx` enum 的 tag checks、实现 unboxed
operand stack 或提高吞吐。未来优化须保留公开输入守卫、source-aware trap 和运行失败后不重跑 Calcit。

## English

Strict admission covers parameters/results, locals, globals, imports, host bindings,
constants, and block/loop/if signatures. It admits Bool, I64, F64, Str, and F64Buffer;
RFC 0004 still excludes F64Buffer globals and constants.

List has no element-type contract. Neither an empty list nor an accidentally homogeneous
runtime list supplies static proof, so all Lists are rejected without recursively inspecting
elements. Nil, Link, and Dynamic are excluded too. Unreachable stack polymorphism does not
relax this program-wide value-domain rule.

Parsed and direct Rust inputs share the CalxProgram gate. Failures preserve function/source
origin under CALX_VALIDATION. ProgramBuilder reuses the type rule and rejects invalid constants
atomically before emission under CALX_PROGRAM_BUILD. CalxProgram checks constant type admission;
F64Buffer is an admitted type, but its constant form is rejected later by the ValidatedProgram
validator (and earlier by the builder). F64Buffer globals are rejected by CalxProgram's global
checks and the builder. Type admission and instruction-specific restrictions are distinct stages.
No runtime branch is added, and entry,
host-result, buffer bounds, and conversion checks remain in place.

Migration from 0.4.0: remove unused Nil constants; model void through zero results and
CalxRunResult::Void; retain separate Uninitialized slots and read-before-write traps. For numeric
batches, construct F64Buffer explicitly at the host entry/import boundary. Existing heterogeneous
programs use the explicit CalxVM::new legacy path or validate/convert at the host boundary.
Current Calcit scalar/F64Buffer lowering already uses the accepted subset; its ABI and dependency
version do not change in this PR.

The CLI still selects its profile from module declarations, so declaration-free legacy programs
may use the compatibility path. Compilers use ProgramBuilder → CalxProgram → ValidatedProgram
→ run_typed directly. This contract establishes an optimization prerequisite; it does not remove
Rust enum tag checks, introduce an unboxed stack, or claim throughput gains. Later optimizations
must preserve input checks, source-aware traps, and no Calcit retry after runtime failure.
