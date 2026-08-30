# ProgramBuilder：从编译器构造严格 Calx 程序

`ProgramBuilder` 是 parser 之外的类型化构造入口，面向 Calcit translator、测试生成器和其他
Rust embedding。它直接产生 `CalxProgram`，不需要先拼接 Cirru 文本，也不会绕过 Calx 的验证与
lowering 阶段。

## 最小路径

```rust
use calx_vm::{
  Calx, CalxBuildError, CalxMutability, CalxProgram, CalxType,
  FunctionBuilder, ProgramBuilder, ValidatedProgram,
};

fn build_program() -> Result<CalxProgram, CalxBuildError> {
  let mut program = ProgramBuilder::new();
  let base = program.global(
    "$base",
    CalxType::F64,
    CalxMutability::Const,
    Calx::F64(40.0),
  )?;

  let mut main = FunctionBuilder::synthetic(
    "main",
    vec![CalxType::F64],
    "generated:calcit/app.main/main",
  )?;
  main
    .body()
    .global_get(&base)?
    .constant(Calx::F64(2.0))?
    .emit(calx_vm::CalxSyntax::Add)?
    .return_()?;
  program.function(main)?;
  program.build()
}

let program = build_program()?;
let validated = ValidatedProgram::try_from_program(program)?;
```

最后一行是安全边界的一部分。`ProgramBuilder::build` 只检查声明图的基本 contract，返回的仍是
未验证程序；operand stack、control stack、调用签名和 lowering 必须由
`ValidatedProgram::try_from_program` 检查。也可以直接调用 `CalxVM::from_program`，它走同一条
验证路径。

## 声明与 handle

- `ProgramBuilder::global` 返回 `GlobalId`；`import` 返回 `ImportId`。
- `FunctionBuilder::parameter` 和 `local` 返回属于该函数的 `LocalId`。
- local/global/import 的指令 API 只接受对应 opaque handle。通用 `emit` 不接受裸索引或裸 import
  名称，因此不能意外引用另一个 builder 的声明。
- handle 只 clone 小型索引、类型和 `Rc` owner token，不 clone 完整声明或函数体。
- 参数与 local 必须在首条 instruction 之前声明；重复名称、跨 builder handle、const global 写入、
  空名称和严格边界上的 `Nil` 都返回 `CalxBuildError`。
- builder API 没有 `Dynamic` 类型入口，不会为缺失的类型或初始值进行隐式推断。

`CalxBuildError` 使用稳定诊断 code `CALX_PROGRAM_BUILD` 和 `build` phase。失败操作不会提交部分
声明或半个 structured control region，因此 translator 可以把错误直接映射成整个 closed call
graph 的 fallback。

## Structured control

`BodyBuilder::block`、`loop_` 和 `if_else` 接受闭包，并计算 parser 所使用的 canonical flattened
instruction targets。调用方不应直接构造 `Block`、`If`、`BlockEnd`、`ElseEnd` 或 `ThenEnd`。

```rust
body.if_else(
  vec![CalxType::F64],
  |then_body| {
    then_body.local_get(&value)?;
    Ok(())
  },
  |else_body| {
    else_body.constant(Calx::F64(0.0))?;
    Ok(())
  },
)?;
```

两个 branch 先在临时 section 中构造；任意闭包返回错误时，父 body 保持不变。每个 structured
region 有一次临时 `Vec` 成本，普通 instruction 只追加到预分配增长的 syntax/span vectors，不为
每条 instruction 建 AST 节点或解析字符串。

## Source origin

- parser 提供的真实位置可传给 `global_at`、`import_at`、`local_at`、`emit_at` 和 structured
  control 的 `*_at` 方法。
- 生成代码可使用 `FunctionBuilder::synthetic` 和 `SourceSpan::synthetic`，形成稳定的
  `source:1:1` origin。
- translator 连续发射不同来源的 handle instruction 时，可用 `BodyBuilder::set_default_span` 更新
  后续 instruction 的位置；它不创建临时 AST。

验证错误和 runtime trap 会继续携带这些 span，所以 parser 路径与 builder 路径可以产生一致的
source diagnostic。

## 兼容性说明

这是新增的 Rust source API，不改变现有 Cirru parser、CLI 格式或 legacy dynamic embedding。
`CalxProgram` 与 builder handle 都不是稳定的序列化格式；0.3 开发期间允许在 semver minor 版本中
继续收紧方法和诊断字段。需要长期保存的编译产物应保存 Calcit source/typed snapshot 与 toolchain
版本，并在加载时重新构造、验证和 lowering。

对 Calcit → Calx 首批子集，建议严格遵循：

`typed preprocessed snapshot -> eligibility -> ProgramBuilder -> CalxProgram -> ValidatedProgram`

不要从原始 Cirru 重新推断类型，不要把 `Number` 猜成 `I64`，也不要用 `Nil` 或 dynamic host
contract 代替无法证明的类型。详细子集和 fallback 契约见
[`RFC 0003`](../RFCs/0003-calcit-subset-and-host-abi.md)。
