use calx_vm::{parse_program, Calx, CalxHostBindings, CalxRunResult, CalxVM, DiagnosticCode};

type TestResult<T = ()> = Result<T, String>;

fn strict_vm(source: &str) -> TestResult<CalxVM> {
  let program = parse_program("wasm-mapping.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  CalxVM::from_program(program, CalxHostBindings::new()).map_err(|error| error.to_string())
}

fn run(source: &str, args: Vec<Calx>) -> TestResult<CalxRunResult> {
  strict_vm(source)?.run_typed(args).map_err(|error| error.to_string())
}

#[test]
fn shared_i64_operations_wrap_mask_and_trap_like_wasm() -> TestResult {
  assert_eq!(
    run(
      r#"fn main (-> i64)
  const 9223372036854775807
  const 1
  i.add
  return"#,
      vec![],
    )?,
    CalxRunResult::Value(Calx::I64(i64::MIN))
  );

  assert_eq!(
    run(
      r#"fn main (-> i64)
  const 1
  const 65
  i.shl
  return"#,
      vec![],
    )?,
    CalxRunResult::Value(Calx::I64(2))
  );

  assert_eq!(
    run(
      r#"fn main (-> i64)
  const -7
  const 3
  i.div
  return"#,
      vec![],
    )?,
    CalxRunResult::Value(Calx::I64(-2))
  );

  for (source, expected) in [
    (
      r#"fn main (-> i64)
  const 1
  const 0
  i.div
  return"#,
      "integer divide by zero",
    ),
    (
      r#"fn main (-> i64)
  const -9223372036854775808
  const -1
  i.div
  return"#,
      "integer division overflow",
    ),
  ] {
    let error = strict_vm(source)?.run_typed(vec![]).expect_err("invalid signed division must trap");
    assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
    assert!(error.message.contains(expected), "{error}");
  }

  Ok(())
}

#[test]
fn shared_f64_comparisons_follow_ieee_boundaries() -> TestResult {
  let mut equal = strict_vm(
    r#"fn main (f64 f64 -> bool)
  local.get 0
  local.get 1
  f.eq
  return"#,
  )?;
  assert_eq!(
    equal
      .run_typed(vec![Calx::F64(0.0), Calx::F64(-0.0)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Bool(true))
  );
  assert_eq!(
    equal
      .run_typed(vec![Calx::F64(f64::NAN), Calx::F64(f64::NAN)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Bool(false))
  );

  let mut less_than = strict_vm(
    r#"fn main (f64 f64 -> bool)
  local.get 0
  local.get 1
  f.lt
  return"#,
  )?;
  assert_eq!(
    less_than
      .run_typed(vec![Calx::F64(f64::NEG_INFINITY), Calx::F64(f64::INFINITY)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Bool(true))
  );
  assert_eq!(
    less_than
      .run_typed(vec![Calx::F64(f64::NAN), Calx::F64(1.0)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Bool(false))
  );
  Ok(())
}

#[test]
fn calx_numeric_truthiness_is_an_intentional_control_difference() -> TestResult {
  let mut vm = strict_vm(
    r#"fn main (i64 -> i64)
  local.get 0
  if (-> i64)
    do
      const 11
    do
      const 20
  return"#,
  )?;
  assert_eq!(
    vm.run_typed(vec![Calx::I64(2)]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(11))
  );
  assert_eq!(
    vm.run_typed(vec![Calx::I64(0)]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(20))
  );
  Ok(())
}

#[test]
fn typed_locals_calls_and_tail_calls_preserve_declared_results() -> TestResult {
  assert_eq!(
    run(
      r#"fn add-one (i64 -> i64)
  local.get 0
  const 1
  i.add
  return

fn relay (i64 -> i64)
  local $copy i64
  local.get 0
  local.set $copy
  local.get $copy
  return-call add-one

fn main (-> i64)
  const 41
  call relay
  return"#,
      vec![],
    )?,
    CalxRunResult::Value(Calx::I64(42))
  );

  let mut uninitialized = strict_vm(
    r#"fn main (-> i64)
  local $value i64
  local.get $value
  return"#,
  )?;
  let error = uninitialized
    .run_typed(vec![])
    .expect_err("Calx locals are not implicitly zero initialized");
  assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
  assert!(error.message.contains("read before set for local"), "{error}");
  Ok(())
}

#[test]
fn unreachable_is_a_host_safe_runtime_trap() -> TestResult {
  let mut vm = strict_vm(
    r#"fn main (->)
  unreachable"#,
  )?;
  let error = vm.run_typed(vec![]).expect_err("unreachable must trap");
  assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
  assert!(error.message.contains("unreachable instruction"), "{error}");
  Ok(())
}

#[test]
fn f64_buffer_and_checked_indexing_are_calx_extensions() -> TestResult {
  let mut vm = strict_vm(
    r#"fn main (f64-buffer f64 -> f64)
  local.get 0
  local.get 1
  f64.to-i64-index
  f64-buffer.get
  return"#,
  )?;
  assert_eq!(
    vm.run_typed(vec![Calx::f64_buffer_adopt(vec![3.0, 5.0, 8.0]), Calx::F64(1.0)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::F64(5.0))
  );

  for invalid in [-1.0, 1.5, f64::NAN, f64::INFINITY] {
    let error = vm
      .run_typed(vec![Calx::f64_buffer_adopt(vec![3.0, 5.0, 8.0]), Calx::F64(invalid)])
      .expect_err("invalid Calx buffer index must trap");
    assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
    assert!(error.message.contains("f64.to-i64-index invalid value"), "{invalid}: {error}");
  }
  Ok(())
}
