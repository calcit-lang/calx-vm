# Bounded WebAssembly semantic mapping / 有界 WebAssembly 语义映射

This document classifies only the WebAssembly semantic ideas used by Calx and
the current Calcit-to-Calx kernel compiler. It does not claim source, text,
binary, module, embedding, or testsuite compatibility.

本文只分类 Calx 与当前 Calcit→Calx kernel compiler 实际使用的 WebAssembly
语义概念，不承诺 source、text、binary、module、embedding 或 testsuite 兼容。

## English

### Three distinct layers

1. The **Calcit compiler target** admits statically proven `Number`, `Bool`, and
   `F64Buffer` boundaries. It rejects `Nil`, `Dynamic`, persistent collections,
   nominal values, indirect calls, and unsupported forms before lowering.
2. A **strict Calx program** has declared parameters, results, locals, globals,
   and imports. Its value domain is closed, but the VM still deliberately
   accepts Calx numeric truthiness in control instructions.
3. The **legacy Calx adapter** retains older dynamic source forms for
   compatibility. Its `Nil`, strings, lists, dynamic slots, and teaching output
   are not part of the Calcit compiler-target claim.

The executable evidence named below lives in
`tests/wasm_mapping_tests.rs` unless another test file is stated explicitly.

### Classification

| Class | Meaning |
| --- | --- |
| same | The tested result/trap agrees for the stated, bounded operands. This is not a wider compatibility claim. |
| intentional difference | Calx deliberately exposes a different type, condition, initialization, or embedding rule. |
| Calx extension | The operation has no direct WebAssembly core value/instruction counterpart. |
| out of scope | WebAssembly has the feature, but the Calcit kernel path does not model it. |

### Current Calcit-lowered path

| Calcit/kernel concept | Emitted Calx | Nearest WebAssembly core concept | Class | Executable evidence |
| --- | --- | --- | --- | --- |
| `Number` literal | `const F64` | `f64.const` | same for represented literal bits | `shared_f64_comparisons_follow_ieee_boundaries`; Calcit golden tests own lowering shape |
| `Bool` literal | `const Bool` | integer `const` by convention | intentional difference: Calx has a concrete Bool value; Wasm core has no Bool value type | Calcit scalar golden/differential tests own lowering shape |
| `&+`, `&-`, `&*`, `&/`, unary `&-` | `add`, subtraction sequence, `mul`, `div`, `neg` over F64 | `f64.add/sub/mul/div`, sign negation | same for tested ordinary/IEEE results; NaN payload identity is not promised | opcode matrix float cases |
| `&=`, `&<`, `&>` | `f.eq`, `f.lt`, `f.gt` | `f64.eq/lt/gt` | intentional difference: Calx produces `Bool`; Wasm produces integer `0`/`1` | `shared_f64_comparisons_follow_ieee_boundaries` |
| typed `let`/parameter reference | typed `local.set/get` | `local.set/get` | intentional difference: a Calx local is `Uninitialized` and traps on read-before-set; Wasm defaultable locals are initialized | `typed_locals_calls_and_tail_calls_preserve_declared_results` |
| source sequence / `do` | sequential instruction emission; non-final values are dropped when needed | instruction sequence plus `drop` | same stack intent for the current single-result path | Calcit scalar golden/differential tests own lowering shape |
| `assert-type` / `hint-fn` | erased after static analysis (`hint-fn` contributes Unit/no instruction) | no runtime counterpart | out of scope: source-analysis metadata is not a VM compatibility claim | Calcit eligibility/lowering tests own erasure |
| statically Bool `if` | structured `if`, lowered to checked jumps | `if` | intentional difference: Calcit lowering proves Bool; Calx VM also has numeric truthiness, while Wasm conditions are integer values | `calx_numeric_truthiness_is_an_intentional_control_difference`; Calcit eligibility tests own the Bool proof |
| direct function call | `call` | `call` | same for the tested typed single-result path | `typed_locals_calls_and_tail_calls_preserve_declared_results` |
| tail-position direct call/recur | `return-call` | `return_call` | same direct tail-transfer intent; availability and frame representation are Calx contracts | `typed_locals_calls_and_tail_calls_preserve_declared_results`; `tests/tail_call_tests.rs` |
| no result | `CalxRunResult::Void` | empty function result | same boundary shape; strict Calx does not encode void as `Nil` | `tests/typed_runtime_tests.rs::typed_void_is_not_represented_by_nil` |
| explicit failure | `unreachable` | `unreachable` | same trap intent; Calx returns a source-aware host error instead of panicking | `unreachable_is_a_host_safe_runtime_trap` |
| immutable numeric boundary read | `F64Buffer`, `f64-buffer.get` | no WebAssembly core opaque-buffer value; linear-memory loads are materially different | Calx extension | `f64_buffer_and_checked_indexing_are_calx_extensions`; Calcit buffer differential tests |
| checked Calcit index | `f64.to-i64-index` | numeric conversion instructions | intentional difference: Calx requires finite integral `0 <= value < 2^63`; it neither truncates nor saturates | `f64_buffer_and_checked_indexing_are_calx_extensions` |
| typed host capability | declared `import-fn` plus exact `CalxHostBinding` | imported function | intentional difference: Calx rechecks host values and returns classified host errors with no guest fallback | `tests/typed_runtime_tests.rs` |
| source span, revision cache, eligibility report | compiler/embedding metadata | no WebAssembly core execution instruction | Calx extension | Calcit core owns these contracts |

### VM overlap not emitted as Calcit `Number`

Calcit `Number` lowers to F64, so the current compiler does not use Calx I64
arithmetic for general numeric expressions. Calx still keeps the following
small VM-level overlap deterministic:

| Calx | WebAssembly analogue | Class | Evidence |
| --- | --- | --- | --- |
| `i.add`, `i.mul`, `i.neg` | `i64.add`, `i64.mul`, subtraction from zero | same modulo `2^64` behavior for tested operands | `shared_i64_operations_wrap_mask_and_trap_like_wasm` |
| `i.div`, `i.rem` | signed `i64.div_s`, `i64.rem_s` | same divide-by-zero and signed-division-overflow trap boundary | `shared_i64_operations_wrap_mask_and_trap_like_wasm`; opcode matrix |
| `i.shl`, `i.shr` | `i64.shl`, signed `i64.shr_s` | same masked shift-count behavior for tested operands | `shared_i64_operations_wrap_mask_and_trap_like_wasm` |
| `block`, `loop`, `br`, `br-if` | structured control and labels | same typed-stack/label intent; Calx lowering and internal branch representation differ | opcode and validator matrices |
| numeric truthiness | integer Wasm conditions | intentional difference: Calx treats zero as false and nonzero as true for I64/F64; this is not emitted from a non-Bool Calcit condition | `calx_numeric_truthiness_is_an_intentional_control_difference` |
| VM `f64-buffer.len` (not currently emitted because its I64 result cannot satisfy Calcit Number/F64) | no opaque-buffer counterpart | Calx extension at VM level; out of the current producer subset | `tests/f64_buffer_tests.rs`; tracked by [calcit#889](https://github.com/calcit-lang/calcit/issues/889) |

### Explicitly out of scope

Wasm binaries and WAT parsing, linear memory, tables, indirect/function-reference
calls, multiple results at the Calcit boundary, SIMD, threads, exceptions, GC,
components, Wasm host APIs, and the official testsuite are not Calx compatibility
claims. A future consumer must justify a Calx feature independently; resemblance
to a Wasm instruction is not sufficient.

### Normative references

- [WebAssembly Core Specification 3.0](https://webassembly.github.io/spec/core/)
- [Instruction syntax](https://webassembly.github.io/spec/core/syntax/instructions.html)
- [Instruction validation](https://webassembly.github.io/spec/core/valid/instructions.html)
- [Instruction execution](https://webassembly.github.io/spec/core/exec/instructions.html)
- [Integer and floating-point numerics](https://webassembly.github.io/spec/core/exec/numerics.html)
- [Runtime values and structures](https://webassembly.github.io/spec/core/exec/runtime.html)

### Why no reference runtime is embedded

The current checks are deterministic semantic anchors, not a general
differential engine. Embedding a second runtime or WAT toolchain would add a
dependency and compatibility surface without a demonstrated compiler-target
correctness gap. Unexpected behavior in the mapped intersection should first
be reduced to a small Calx test. A reference runtime may be used in an isolated
investigation, but does not become a core or CI dependency by default.

## 中文

### 三个必须区分的层次

1. **Calcit compiler target** 只允许已经静态证明的 `Number`、`Bool` 与
   `F64Buffer` 边界；`Nil`、`Dynamic`、persistent collections、nominal
   values、间接调用与不支持的 form 在 lowering 前拒绝。
2. **strict Calx program** 声明参数、返回值、local、global 与 import。
   它的值域封闭，但 VM 控制指令仍有意支持 Calx 数值 truthiness。
3. **legacy Calx adapter** 为兼容保留旧动态源码形式。其中的 `Nil`、字符串、
   list、dynamic slot 与教学输出，不属于 Calcit compiler-target 承诺。

下文未特别注明时，可执行证据位于 `tests/wasm_mapping_tests.rs`。

### 分类

| 分类 | 含义 |
| --- | --- |
| same | 在声明的有界操作数范围内，已测结果/trap 一致；不表示更广兼容。 |
| intentional difference | Calx 有意采用不同的类型、条件、初始化或 embedding 规则。 |
| Calx extension | WebAssembly core 没有直接对应的值或指令。 |
| out of scope | WebAssembly 有该能力，但 Calcit kernel 路径不建模。 |

### 当前 Calcit lowering 路径

| Calcit/kernel 概念 | Calx 产物 | 最接近的 WebAssembly core 概念 | 分类 | 可执行证据 |
| --- | --- | --- | --- | --- |
| `Number` literal | `const F64` | `f64.const` | 对可表示 literal bits 为 same | `shared_f64_comparisons_follow_ieee_boundaries`；lowering shape 由 Calcit golden tests 负责 |
| `Bool` literal | `const Bool` | 约定使用整数 `const` | intentional difference：Calx 有 concrete Bool value；Wasm core 没有 Bool value type | lowering shape 由 Calcit scalar golden/differential tests 负责 |
| `&+`、`&-`、`&*`、`&/`、一元 `&-` | F64 上的 `add`、减法序列、`mul`、`div`、`neg` | `f64.add/sub/mul/div` 与符号取反 | 对已测普通/IEEE 结果为 same；不承诺 NaN payload identity | opcode matrix float cases |
| `&=`、`&<`、`&>` | `f.eq`、`f.lt`、`f.gt` | `f64.eq/lt/gt` | intentional difference：Calx 产生 `Bool`，Wasm 产生整数 `0`/`1` | `shared_f64_comparisons_follow_ieee_boundaries` |
| typed `let`/参数引用 | typed `local.set/get` | `local.set/get` | intentional difference：Calx local 初始为 `Uninitialized`，读取会 trap；Wasm defaultable local 会初始化 | `typed_locals_calls_and_tail_calls_preserve_declared_results` |
| source sequence / `do` | 顺序生成指令；必要时丢弃非末尾值 | instruction sequence 加 `drop` | 对当前 single-result path 具有相同 stack intent | lowering shape 由 Calcit scalar golden/differential tests 负责 |
| `assert-type` / `hint-fn` | 静态分析后擦除（`hint-fn` 贡献 Unit/无指令） | 无运行时对应 | out of scope：source-analysis metadata 不属于 VM 兼容承诺 | erasure 由 Calcit eligibility/lowering tests 负责 |
| 已静态证明为 Bool 的 `if` | structured `if`，再 lowering 为 checked jumps | `if` | intentional difference：Calcit lowering 证明 Bool；Calx VM 另有 numeric truthiness，而 Wasm condition 是整数值 | `calx_numeric_truthiness_is_an_intentional_control_difference`；Bool 证明由 Calcit eligibility tests 负责 |
| direct function call | `call` | `call` | 对已测 typed single-result path 为 same | `typed_locals_calls_and_tail_calls_preserve_declared_results` |
| tail-position direct call/recur | `return-call` | `return_call` | direct tail transfer 意图相同；可用性与 frame 表示属于 Calx contract | `typed_locals_calls_and_tail_calls_preserve_declared_results`；`tests/tail_call_tests.rs` |
| 无返回值 | `CalxRunResult::Void` | empty function result | boundary shape 相同；strict Calx 不用 `Nil` 编码 void | `tests/typed_runtime_tests.rs::typed_void_is_not_represented_by_nil` |
| 显式失败 | `unreachable` | `unreachable` | trap 意图相同；Calx 返回 source-aware host error，不 panic | `unreachable_is_a_host_safe_runtime_trap` |
| immutable numeric boundary read | `F64Buffer`、`f64-buffer.get` | WebAssembly core 没有 opaque-buffer value；linear-memory load 语义明显不同 | Calx extension | `f64_buffer_and_checked_indexing_are_calx_extensions`；Calcit buffer differential tests |
| checked Calcit index | `f64.to-i64-index` | numeric conversion instructions | intentional difference：Calx 只接受 finite integral 且 `0 <= value < 2^63`，既不 truncation 也不 saturation | `f64_buffer_and_checked_indexing_are_calx_extensions` |
| typed host capability | declared `import-fn` 加精确 `CalxHostBinding` | imported function | intentional difference：Calx 重新检查 host value，返回分类 host error，且 guest 不 fallback | `tests/typed_runtime_tests.rs` |
| source span、revision cache、eligibility report | compiler/embedding metadata | 没有 WebAssembly core execution instruction | Calx extension | 契约由 Calcit core 拥有 |

### 不作为 Calcit `Number` 产生的 VM 交集

Calcit `Number` lowering 为 F64，因此当前 compiler 不使用 Calx I64 算术表达
普通数值计算。Calx 仍将下面的小型 VM-level 交集保持为确定性语义：

| Calx | WebAssembly 对应 | 分类 | 证据 |
| --- | --- | --- | --- |
| `i.add`、`i.mul`、`i.neg` | `i64.add`、`i64.mul`、从零相减 | 对已测操作数具有相同 modulo `2^64` 行为 | `shared_i64_operations_wrap_mask_and_trap_like_wasm` |
| `i.div`、`i.rem` | signed `i64.div_s`、`i64.rem_s` | divide-by-zero 与 signed-division-overflow trap 边界相同 | `shared_i64_operations_wrap_mask_and_trap_like_wasm`；opcode matrix |
| `i.shl`、`i.shr` | `i64.shl`、signed `i64.shr_s` | 对已测操作数具有相同 masked shift-count 行为 | `shared_i64_operations_wrap_mask_and_trap_like_wasm` |
| `block`、`loop`、`br`、`br-if` | structured control 与 label | typed-stack/label 意图相同；Calx lowering 与内部 branch 表示不同 | opcode/validator matrices |
| numeric truthiness | Wasm integer condition | intentional difference：Calx 以 I64/F64 的零为 false、非零为 true；Calcit compiler 不从非 Bool condition 产生该路径 | `calx_numeric_truthiness_is_an_intentional_control_difference` |
| VM `f64-buffer.len`（当前不生成，因为 I64 结果无法满足 Calcit Number/F64） | 无 opaque-buffer 对应 | VM 层是 Calx extension；不在当前 producer 子集 | `tests/f64_buffer_tests.rs`；由 [calcit#889](https://github.com/calcit-lang/calcit/issues/889) 追踪 |

### 明确不在范围内

Wasm binary/WAT parsing、linear memory、table、indirect/function-reference
call、Calcit 边界多返回值、SIMD、thread、exception、GC、component、Wasm host
API 与官方 testsuite 都不是 Calx 兼容承诺。未来 consumer 必须独立证明新增 Calx
能力的必要性；仅仅类似某条 Wasm 指令并不构成理由。

### 规范引用

- [WebAssembly Core Specification 3.0](https://webassembly.github.io/spec/core/)
- [Instruction syntax](https://webassembly.github.io/spec/core/syntax/instructions.html)
- [Instruction validation](https://webassembly.github.io/spec/core/valid/instructions.html)
- [Instruction execution](https://webassembly.github.io/spec/core/exec/instructions.html)
- [Integer and floating-point numerics](https://webassembly.github.io/spec/core/exec/numerics.html)
- [Runtime values and structures](https://webassembly.github.io/spec/core/exec/runtime.html)

### 为什么不嵌入 reference runtime

当前检查是确定性的语义锚点，不是通用 differential engine。在没有实际
compiler-target correctness gap 时，嵌入第二套 runtime 或 WAT toolchain 只会增加
依赖与兼容面。映射交集中的异常行为应先缩减为小型 Calx test。隔离调查可以临时使用
reference runtime，但默认不进入 core 或 CI dependency。
