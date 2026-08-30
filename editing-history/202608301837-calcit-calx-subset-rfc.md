# Calcit → Calx 子集与 ABI RFC

## 修改概要

- 新增 RFC 0003，冻结 Calcit typed scalar kernel 的类型、语法、调用闭包与 all-or-nothing
  fallback 契约。
- 明确 Calcit `Number -> Calx F64`、`Bool -> Bool`、函数级 `Unit -> void`；首版拒绝 Nil、
  Dynamic、容器、global/ref、closure/HOF 与 rest/optional ABI。
- 定义 `calcit-calx-kernel/1` entry/host ABI、ownership、VM 重复调用生命周期与 capability 规则。
- 定义结构化 fallback codes、golden kernel/corpus 和端到端性能计量阶段。
- 识别 F64 comparisons 为 range-sum/Fibonacci translator 的前置指令集缺口，并建立 #45 独立追踪。
- 更新 README RFC 索引与 M3 路线图，移除把 Calcit source number/nil 当作首版 Calx i64/nil
  边界的旧表述。

## 关键知识点

- Calcit frontend 已有 `CompiledProgram`、`CompiledDef.preprocessed_code`、`DefId/deps/schema` 和
  `CalcitFn` 类型/arity 元数据，适合在 Calcit 侧产生 Calx target；`calx_vm` 不应反向依赖 Calcit。
- Calcit truthiness 与 Calx legacy truthiness 对 `0` 的规则不同，compiler subset 必须静态要求
  Bool condition。
- Calcit runtime `Number` 是 `f64`；即使字面量看似整数也不能推断为 I64。
- 当前 Calx 只有 I64 comparisons，没有 F64 comparisons；减法可由 `Neg + Add` 表达。
- compiler fallback 必须发生在执行选择阶段。VM trap 后自动重跑会重复 effect import，也会掩盖
  validator/runtime bug。
- 现有 Calcit Wasm emitter 对部分不可编译非导出函数保留默认值 body；Calx strict compiler 不得
  复制该 placeholder 行为。

## 验证

- `cargo fmt --check`
- `cargo test`：59 passed
- `./try.sh`：9 demos passed
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo package --allow-dirty --offline`
