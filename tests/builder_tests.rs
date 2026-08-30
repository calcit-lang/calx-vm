use std::{error::Error, fmt, rc::Rc};

use calx_vm::{
  parse_program, Calx, CalxBuildError, CalxBuildErrorKind, CalxError, CalxHostBinding, CalxHostBindings, CalxMutability, CalxProgram,
  CalxRunResult, CalxSyntax, CalxType, CalxVM, DiagnosticCode, DiagnosticPhase, FunctionBuilder, ProgramBuilder, SourceSpan,
  ValidatedProgram,
};

#[derive(Debug)]
struct TestError(String);

impl fmt::Display for TestError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl Error for TestError {}

impl From<String> for TestError {
  fn from(value: String) -> Self {
    Self(value)
  }
}

impl From<CalxBuildError> for TestError {
  fn from(value: CalxBuildError) -> Self {
    Self(value.to_string())
  }
}

type TestResult<T> = Result<T, TestError>;

fn add2(values: &[Calx]) -> Result<Calx, CalxError> {
  let [Calx::F64(left), Calx::F64(right)] = values else {
    return Err(CalxError::new_raw("add2 expected two f64 values".to_string()));
  };
  Ok(Calx::F64(left + right))
}

fn notify(values: &[Calx]) -> Result<(), CalxError> {
  if matches!(values, [Calx::F64(_)]) {
    Ok(())
  } else {
    Err(CalxError::new_raw("notify expected one f64 value".to_string()))
  }
}

fn bindings() -> TestResult<CalxHostBindings> {
  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("notify"),
    CalxHostBinding::void(vec![CalxType::F64], notify).map_err(|error| error.to_string())?,
  );
  bindings.insert(
    Rc::from("add2"),
    CalxHostBinding::value(vec![CalxType::F64, CalxType::F64], CalxType::F64, add2).map_err(|error| error.to_string())?,
  );
  Ok(bindings)
}

fn build_equivalent_program() -> TestResult<CalxProgram> {
  let mut program = ProgramBuilder::new();
  let base = program
    .global("$base", CalxType::F64, CalxMutability::Mut, Calx::F64(40.0))
    .map_err(|error| error.to_string())?;
  let notify = program
    .import("notify", vec![CalxType::F64], None)
    .map_err(|error| error.to_string())?;
  let add2 = program
    .import("add2", vec![CalxType::F64, CalxType::F64], Some(CalxType::F64))
    .map_err(|error| error.to_string())?;

  let mut positive = FunctionBuilder::new("positive", vec![CalxType::F64]).map_err(|error| error.to_string())?;
  let value = positive.parameter("$value", CalxType::F64).map_err(|error| error.to_string())?;
  positive
    .body()
    .block(vec![], vec![CalxType::F64], |body| {
      body.local_get(&value)?.constant(Calx::F64(0.0))?.emit(CalxSyntax::F64Gt)?;
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
      Ok(())
    })?
    .return_()?;
  program.function(positive).map_err(|error| error.to_string())?;

  let mut loop_id = FunctionBuilder::new("loop-id", vec![CalxType::F64]).map_err(|error| error.to_string())?;
  let value = loop_id.parameter("$value", CalxType::F64).map_err(|error| error.to_string())?;
  loop_id.body().local_get(&value)?;
  loop_id
    .body()
    .loop_(vec![CalxType::F64], vec![CalxType::F64], |body| {
      body.emit(CalxSyntax::Nop)?;
      Ok(())
    })?
    .return_()?;
  program.function(loop_id).map_err(|error| error.to_string())?;

  let mut main = FunctionBuilder::new("main", vec![CalxType::F64]).map_err(|error| error.to_string())?;
  let answer = main.local("$answer", CalxType::F64).map_err(|error| error.to_string())?;
  main.body().global_get(&base)?.constant(Calx::F64(2.0))?;
  main.body().call_import(&add2)?.local_tee(&answer)?.call_import(&notify)?;
  main.body().local_get(&answer)?.call("positive")?.call("loop-id")?.return_()?;
  program.function(main).map_err(|error| error.to_string())?;
  Ok(program.build()?)
}

#[test]
fn builder_matches_parser_validation_lowering_and_execution() -> TestResult<()> {
  let source = r#"global $base (mut f64) 40.
import-fn notify (f64 ->)
import-fn add2 (f64 f64 -> f64)

fn positive (($value f64) -> f64)
  block (-> f64)
    local.get $value
    const 0.
    f.gt
    if (-> f64)
      do
        local.get $value
      do
        const 0.
  return

fn loop-id (($value f64) -> f64)
  local.get $value
  loop (f64 -> f64)
    nop
  return

fn main (-> f64)
  local $answer f64
  global.get $base
  const 2.
  call-import add2
  local.tee $answer
  call-import notify
  local.get $answer
  call positive
  call loop-id
  return"#;
  let parsed_program = parse_program("builder-equivalent.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  let built_program = build_equivalent_program()?;

  assert_eq!(built_program.globals().len(), parsed_program.globals().len());
  assert_eq!(built_program.imports().len(), parsed_program.imports().len());
  assert_eq!(built_program.functions().len(), parsed_program.functions().len());
  for (built, parsed) in built_program.globals().iter().zip(parsed_program.globals()) {
    assert_eq!(
      (&built.name, built.value_type, built.mutability, &built.initial),
      (&parsed.name, parsed.value_type, parsed.mutability, &parsed.initial)
    );
  }
  for (built, parsed) in built_program.imports().iter().zip(parsed_program.imports()) {
    assert_eq!(
      (&built.name, &built.params, built.result),
      (&parsed.name, &parsed.params, parsed.result)
    );
  }
  for (built, parsed) in built_program.functions().iter().zip(parsed_program.functions()) {
    assert_eq!(
      (&built.name, &built.params_types, &built.ret_types),
      (&parsed.name, &parsed.params_types, &parsed.ret_types)
    );
    assert_eq!(built.local_names, parsed.local_names);
    assert_eq!(built.syntax, parsed.syntax);
  }

  let built_validated = ValidatedProgram::try_from_program(built_program.clone()).map_err(|error| error.to_string())?;
  let parsed_validated = ValidatedProgram::try_from_program(parsed_program.clone()).map_err(|error| error.to_string())?;
  assert_eq!(built_validated.functions().len(), parsed_validated.functions().len());
  for (built, parsed) in built_validated.functions().iter().zip(parsed_validated.functions()) {
    assert_eq!(built.instrs, parsed.instrs);
  }

  let mut built_vm = CalxVM::from_program(built_program, bindings()?).map_err(|error| error.to_string())?;
  let mut parsed_vm = CalxVM::from_program(parsed_program, bindings()?).map_err(|error| error.to_string())?;
  let expected = CalxRunResult::Value(Calx::F64(42.0));
  assert_eq!(built_vm.run_typed(vec![]).map_err(|error| error.to_string())?, expected);
  assert_eq!(parsed_vm.run_typed(vec![]).map_err(|error| error.to_string())?, expected);
  Ok(())
}

#[test]
fn builder_rejects_nil_duplicates_foreign_handles_and_partial_structures() -> TestResult<()> {
  let mut program = ProgramBuilder::new();
  let error = program
    .global("$nil", CalxType::Nil, CalxMutability::Mut, Calx::Nil)
    .expect_err("Nil cannot enter a strict builder boundary");
  assert_eq!(error.kind, CalxBuildErrorKind::InvalidType);
  assert_eq!(error.diagnostic().code, DiagnosticCode::ProgramBuild);
  assert_eq!(error.diagnostic().phase, DiagnosticPhase::Build);

  let global = program
    .global("$value", CalxType::I64, CalxMutability::Const, Calx::I64(1))
    .map_err(|error| error.to_string())?;
  let error = program
    .global("$value", CalxType::I64, CalxMutability::Mut, Calx::I64(2))
    .expect_err("duplicate globals must not replace the first declaration");
  assert_eq!(error.kind, CalxBuildErrorKind::DuplicateDeclaration);
  assert_eq!(global.index(), 0);

  let mut function = FunctionBuilder::new("main", vec![]).map_err(|error| error.to_string())?;
  let local = function.parameter("$value", CalxType::I64).map_err(|error| error.to_string())?;
  assert_eq!(
    function.local("$value", CalxType::I64).expect_err("duplicate local").kind,
    CalxBuildErrorKind::DuplicateDeclaration
  );
  assert_eq!(
    function.body().global_set(&global).expect_err("const global write").kind,
    CalxBuildErrorKind::InvalidInstruction
  );
  assert!(function.body().is_empty());
  function.body().local_get(&local).map_err(|error| error.to_string())?;
  assert_eq!(
    function.local("$late", CalxType::I64).expect_err("late local").kind,
    CalxBuildErrorKind::InvalidDeclarationOrder
  );

  let mut structure = FunctionBuilder::new("structure", vec![]).map_err(|error| error.to_string())?;
  let error = structure
    .body()
    .block(vec![], vec![], |body| {
      body.emit(CalxSyntax::BlockEnd(false))?;
      Ok(())
    })
    .expect_err("failed child construction must not commit a partial block");
  assert_eq!(error.kind, CalxBuildErrorKind::InvalidInstruction);
  assert!(structure.body().is_empty());

  let mut other_function = FunctionBuilder::new("other", vec![]).map_err(|error| error.to_string())?;
  assert_eq!(
    other_function.body().local_get(&local).expect_err("foreign local").kind,
    CalxBuildErrorKind::ForeignHandle
  );

  let mut other_program = ProgramBuilder::new();
  other_program
    .function(function)
    .expect("a failed const write must not retain the global handle owner");

  let mut foreign_function = FunctionBuilder::new("foreign", vec![]).map_err(|error| error.to_string())?;
  foreign_function.body().global_get(&global)?;
  assert_eq!(
    other_program.function(foreign_function).expect_err("foreign module handle").kind,
    CalxBuildErrorKind::ForeignHandle
  );
  assert_eq!(other_program.build()?.functions().len(), 1);
  assert_eq!(program.build()?.globals().len(), 1);

  let mut raw = FunctionBuilder::new("raw", vec![]).map_err(|error| error.to_string())?;
  assert_eq!(
    raw
      .body()
      .emit(CalxSyntax::LocalGet(0))
      .expect_err("raw indexed local access must use a LocalId")
      .kind,
    CalxBuildErrorKind::InvalidInstruction
  );
  assert_eq!(
    raw.body().call("").expect_err("empty call target").kind,
    CalxBuildErrorKind::InvalidName
  );
  assert!(raw.body().is_empty());
  Ok(())
}

#[test]
fn builder_preserves_source_backed_and_synthetic_diagnostics() -> TestResult<()> {
  let source = "fn main (-> f64)\n  const true\n  neg\n  return";
  let parsed = parse_program("source-backed.cirru", source).map_err(|error| error.to_string())?;
  let spans = parsed.functions[0].source_spans.clone();
  let parsed_program = parsed.into_program().map_err(|error| error.to_string())?;

  let mut program = ProgramBuilder::new();
  let mut main = FunctionBuilder::new("main", vec![CalxType::F64]).map_err(|error| error.to_string())?;
  main
    .body()
    .emit_at(CalxSyntax::Const(Calx::Bool(true)), spans[0].clone().expect("const span"))?
    .emit_at(CalxSyntax::Neg, spans[1].clone().expect("neg span"))?
    .emit_at(CalxSyntax::Return, spans[2].clone().expect("return span"))?;
  program.function(main).map_err(|error| error.to_string())?;
  let built_program = program.build().map_err(|error| error.to_string())?;

  let parsed_error = ValidatedProgram::try_from_program(parsed_program).expect_err("parser program must reject Bool negation");
  let built_error = ValidatedProgram::try_from_program(built_program).expect_err("builder program must reject Bool negation");
  assert_eq!(built_error.validation_error(), parsed_error.validation_error());

  let mut generated = ProgramBuilder::new();
  let mut main =
    FunctionBuilder::synthetic("main", vec![CalxType::F64], "generated:calcit/app.math/bad").map_err(|error| error.to_string())?;
  main.body().constant(Calx::Bool(true))?.emit(CalxSyntax::Neg)?.return_()?;
  generated.function(main).map_err(|error| error.to_string())?;
  let error = ValidatedProgram::try_from_program(generated.build().map_err(|error| error.to_string())?)
    .expect_err("synthetic invalid code must retain its generated origin");
  let span = error.span.as_deref().expect("synthetic validation span");
  assert_eq!(span, &SourceSpan::synthetic("generated:calcit/app.math/bad"));
  assert_eq!(span.location(), "generated:calcit/app.math/bad:1:1");
  Ok(())
}

#[test]
fn builder_and_parser_traps_keep_the_same_source_diagnostic() -> TestResult<()> {
  let source = "fn main (->)\n  unreachable";
  let parsed = parse_program("trap.cirru", source).map_err(|error| error.to_string())?;
  let trap_span = parsed.functions[0].source_spans[0].clone().expect("unreachable span");
  let parsed_program = parsed.into_program().map_err(|error| error.to_string())?;

  let mut program = ProgramBuilder::new();
  let mut main = FunctionBuilder::new("main", vec![]).map_err(|error| error.to_string())?;
  main.body().emit_at(CalxSyntax::Unreachable, trap_span.clone())?;
  program.function(main).map_err(|error| error.to_string())?;
  let built_program = program.build().map_err(|error| error.to_string())?;

  let mut parsed_vm = CalxVM::from_program(parsed_program, CalxHostBindings::new()).map_err(|error| error.to_string())?;
  let mut built_vm = CalxVM::from_program(built_program, CalxHostBindings::new()).map_err(|error| error.to_string())?;
  let parsed_error = parsed_vm.run_typed(vec![]).expect_err("unreachable must trap");
  let built_error = built_vm.run_typed(vec![]).expect_err("unreachable must trap");
  assert_eq!(built_error.code(), parsed_error.code());
  assert_eq!(built_error.message, parsed_error.message);
  assert_eq!(built_error.source_span(), parsed_error.source_span());
  assert_eq!(built_error.source_span(), Some(&trap_span));
  Ok(())
}
