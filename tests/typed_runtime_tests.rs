use std::rc::Rc;

use calx_vm::{
  parse_program, Calx, CalxError, CalxHostBinding, CalxHostBindings, CalxInstr, CalxProgram, CalxRunResult, CalxType, CalxVM,
  DiagnosticCode, ValidatedProgram,
};

fn add2(values: &[Calx]) -> Result<Calx, CalxError> {
  let [Calx::I64(a), Calx::I64(b)] = values else {
    return Err(CalxError::new_raw("add2 expected two i64 values".to_string()));
  };
  Ok(Calx::I64(a + b))
}

fn notify(values: &[Calx]) -> Result<(), CalxError> {
  if matches!(values, [Calx::I64(_)]) {
    Ok(())
  } else {
    Err(CalxError::new_raw("notify expected one i64 value".to_string()))
  }
}

fn wrong_result(_values: &[Calx]) -> Result<Calx, CalxError> {
  Ok(Calx::Bool(true))
}

fn strict_program(source: &str) -> Result<CalxProgram, String> {
  parse_program("typed-runtime.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())
}

#[test]
fn executes_typed_locals_globals_and_indexed_imports() -> Result<(), String> {
  let source = r#"global $base (mut i64) 40
import-fn notify (i64 ->)
import-fn add2 (i64 i64 -> i64)

fn main (-> i64)
  local $answer i64
  const 5
  call-import notify
  global.get $base
  const 2
  call-import add2
  local.set $answer
  local.get $answer
  return"#;
  let program = strict_program(source)?;
  let validated = ValidatedProgram::try_from_program(program.clone()).map_err(|error| error.to_string())?;
  assert!(matches!(validated.functions()[0].instrs[1], CalxInstr::CallImportIndexed(0)));
  assert!(matches!(validated.functions()[0].instrs[4], CalxInstr::CallImportIndexed(1)));

  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("notify"),
    CalxHostBinding::void(vec![CalxType::I64], notify).map_err(|error| error.to_string())?,
  );
  bindings.insert(
    Rc::from("add2"),
    CalxHostBinding::value(vec![CalxType::I64, CalxType::I64], CalxType::I64, add2).map_err(|error| error.to_string())?,
  );
  let mut vm = CalxVM::from_program(program, bindings).map_err(|error| error.to_string())?;
  assert_eq!(
    vm.run_typed(vec![]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(42))
  );
  Ok(())
}

#[test]
fn typed_void_is_not_represented_by_nil() -> Result<(), String> {
  let program = strict_program("fn main (->)\n  return")?;
  let mut vm = CalxVM::from_program(program, CalxHostBindings::new()).map_err(|error| error.to_string())?;
  assert_eq!(vm.run_typed(vec![]).map_err(|error| error.to_string())?, CalxRunResult::Void);
  Ok(())
}

#[test]
fn typed_local_read_before_set_traps_without_nil_placeholder() -> Result<(), String> {
  let program = strict_program("fn main (-> i64)\n  local $value i64\n  local.get $value\n  return")?;
  let mut vm = CalxVM::from_program(program, CalxHostBindings::new()).map_err(|error| error.to_string())?;
  let error = vm.run_typed(vec![]).expect_err("uninitialized local must trap");
  assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
  assert!(error.message.contains("read before set for local"), "{error}");
  Ok(())
}

#[test]
fn strict_validation_rejects_const_global_write() -> Result<(), String> {
  let program = strict_program("global $build (const i64) 1\nfn main (->)\n  const 2\n  global.set $build\n  return")?;
  let error = ValidatedProgram::try_from_program(program).expect_err("const global writes must fail before execution");
  assert!(error.message.contains("cannot write const global"), "{error}");
  assert_eq!(error.function.as_deref(), Some("main"));
  Ok(())
}

#[test]
fn host_signature_mismatch_fails_before_execution() -> Result<(), String> {
  let program =
    strict_program("import-fn add2 (i64 i64 -> i64)\nfn main (-> i64)\n  const 1\n  const 2\n  call-import add2\n  return")?;
  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("add2"),
    CalxHostBinding::value(vec![CalxType::I64], CalxType::I64, add2).map_err(|error| error.to_string())?,
  );
  let error = CalxVM::from_program(program, bindings).expect_err("guest and host signatures must match");
  assert!(error.message.contains("signature mismatch"), "{error}");
  Ok(())
}

#[test]
fn host_result_mismatch_stays_at_the_host_boundary() -> Result<(), String> {
  let program = strict_program("import-fn read (-> i64)\nfn main (-> i64)\n  call-import read\n  return")?;
  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("read"),
    CalxHostBinding::value(vec![], CalxType::I64, wrong_result).map_err(|error| error.to_string())?,
  );
  let mut vm = CalxVM::from_program(program, bindings).map_err(|error| error.to_string())?;
  let error = vm.run_typed(vec![]).expect_err("host result must be checked");
  assert_eq!(error.code(), DiagnosticCode::HostImport);
  assert!(error.snapshot.is_none());
  assert!(error.message.contains("result expected I64, found Bool"), "{error}");
  Ok(())
}

#[test]
fn legacy_local_new_uses_uninitialized_state_instead_of_nil() -> Result<(), String> {
  let parsed = parse_program(
    "legacy-uninitialized.cirru",
    "fn main (-> i64)\n  local.new $value\n  local.get $value\n  return",
  )
  .map_err(|error| error.to_string())?;
  let mut vm = CalxVM::new(parsed.functions, vec![], Default::default());
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  let error = vm.run(vec![]).expect_err("legacy read-before-set must trap");
  assert!(error.message.contains("read before set for local"), "{error}");
  Ok(())
}
