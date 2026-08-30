# ProgramBuilder 与 source-aware typed construction

## 本次修改

- 新增 `ProgramBuilder`、`FunctionBuilder`、`BodyBuilder`，让 Calcit translator 和 Rust
  embedding 直接构造 `CalxProgram`，不需要生成 Cirru 文本。
- 用 `LocalId`、`GlobalId`、`ImportId` 封装 declaration reference；owner token 阻止跨函数或跨
  module builder 混用 handle。
- builder 严格边界不接受 `Nil`，且 API 不暴露隐式 `Dynamic`；重复声明、initializer mismatch、
  const global 写入、错误声明顺序等均返回结构化 `CalxBuildError`。
- structured `block`、`loop_`、`if_else` 在临时 section 内完成，成功后一次性 rebase/commit，避免
  失败时留下半个 control region。
- `ProgramBuilder::build` 只产生未验证 `CalxProgram`；operand/control stack validation 与 lowering
  继续由 `ValidatedProgram::try_from_program` 或 `CalxVM::from_program` 完成。
- 新增 `CALX_PROGRAM_BUILD` / `build` diagnostic，并支持真实 `SourceSpan` 与稳定 synthetic origin。
- 增加 parser/builder 在 declaration、syntax、validation、lowering、execution、trap 和 source
  diagnostic 上的等价回归测试。

## 实现知识点

- parser 的 flattened `if` metadata 中，`else_at` 指向 then branch 起点，而 `to` 是 `ThenEnd`
  之后的位置；相对 section 长度计算分别是 `else_len + 2` 与
  `else_len + then_len + 3`。少算 1 可能在简单验证中不暴露，但会造成 parser/builder lowering
  不等价。
- 失败操作不能只保证 syntax vector 不变，还要保证 owner 等隐藏 metadata 不变。尤其 const
  global 写入必须先检查 mutability，再采用 module owner；structured append 也应先完成 owner
  compatibility 与 index rebase 检查，最后再提交 owner 和 vectors。
- 通用 `emit` 不应接收裸 local/global index 或裸 import name，否则会绕过 opaque handle 的 scope
  检查。primitive stack instruction 可以直接 emit，declaration-sensitive instruction 通过专用方法
  push。
- 每条普通 instruction 仅追加到 syntax/span vectors；`SourceSpan` clone 只复制位置和 `Rc<str>`。
  structured region 为保证原子性按 region 使用临时 vectors，不在 VM 执行热路径引入 builder
  allocation。

## 验证

- `cargo fmt --check`
- `cargo test`
- `cargo test --release`
- `./try.sh`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo package --allow-dirty --offline`
