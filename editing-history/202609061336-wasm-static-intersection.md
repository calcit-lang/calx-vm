# Bound the WebAssembly semantic intersection / 收紧 WebAssembly 语义交集

## English

- Rewrite issue #34 around the current product boundary: Calx is a strict Calcit kernel executor, not a Wasm-compatible runtime.
- Add a bilingual mapping that separates the Calcit compiler target, strict Calx programs, and the legacy adapter, and classifies each exercised semantic area as same, intentional difference, Calx extension, or out of scope.
- Add six deterministic strict-program tests for integer wrapping/traps, IEEE F64 comparisons, numeric truthiness, typed locals/calls/tail calls, host-safe `unreachable`, and checked `F64Buffer` indexing.
- Link only official WebAssembly Core Specification rules; do not add a reference runtime, WAT parser, binary tooling, dependency, opcode, public API, or generated report.
- Correct stale README wording now that 0.5.0 is published, and clarify Dynamic/Nil/legacy wording in the existing instruction docs.
- Record the audit finding that VM `f64-buffer.len` produces I64 while Calcit exposes `Number`/F64; keep it outside the current producer subset and link calcit#889 instead of claiming a working lowering.

## 中文

- 按当前产品边界重写 #34：Calx 是严格的 Calcit kernel executor，不是 Wasm-compatible runtime。
- 新增双语映射，区分 Calcit compiler target、strict Calx program 与 legacy adapter，并把实际语义逐项分类为 same、intentional difference、Calx extension 或 out of scope。
- 新增六项确定性 strict-program tests，覆盖整数 wrapping/trap、IEEE F64 comparison、numeric truthiness、typed local/call/tail call、host-safe `unreachable` 与 checked `F64Buffer` indexing。
- 只链接 WebAssembly Core Specification 官方规则；不增加 reference runtime、WAT parser、binary tooling、dependency、opcode、公共 API 或 generated report。
- 0.5.0 已发布，因此修正 README 的陈旧状态，并在现有 instruction docs 中澄清 Dynamic/Nil/legacy 分层。
- 记录审计发现：VM `f64-buffer.len` 产生 I64，而 Calcit 公开为 `Number`/F64；将其排除在当前 producer 子集外并链接 calcit#889，不再误称 lowering 已可用。
