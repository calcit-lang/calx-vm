# Complete WebAssembly mapping evidence / 补齐 WebAssembly 映射证据

## English

PR #68 review found four small but valid gaps between the mapping and its executable evidence. The
VM-overlap table had a five-cell `f64-buffer.len` row under a four-column header. The I64 test cited
`i.mul`, `i.neg`, and `i.shr` without exercising them, numeric truthiness named F64 but only tested
I64, and the checked-index test omitted the exact rejected `2^63` upper bound.

The follow-up keeps the scope unchanged and closes those evidence gaps with deterministic cases. It
does not add instructions, dependencies, a reference runtime, or broader compatibility claims.

## 中文

PR #68 review 发现映射与可执行证据之间有四个小而有效的缺口：VM overlap 表格在四列表头下写了
五格的 `f64-buffer.len` 行；I64 测试引用了 `i.mul`、`i.neg`、`i.shr` 却没有执行；numeric
truthiness 声明覆盖 F64，但只测试了 I64；checked-index 测试遗漏了恰好等于 `2^63` 的拒绝上界。

本次跟进不扩大范围，只用确定性 case 补齐这些证据；没有新增指令、依赖、reference runtime 或更宽的
兼容性承诺。
