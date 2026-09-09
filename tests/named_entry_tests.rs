use std::rc::Rc;

use calx_vm::{
  parse_program, Calx, CalxError, CalxHostBinding, CalxHostBindings, CalxRunResult, CalxTraceError, CalxType, CalxVM, DiagnosticCode,
  VmEvent, VmObserver,
};

fn strict_vm(source: &str, bindings: CalxHostBindings) -> Result<CalxVM, String> {
  let program = parse_program("named-entry.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  CalxVM::from_program(program, bindings).map_err(|error| error.to_string())
}

#[derive(Default)]
struct Events(Vec<VmEvent>);

impl VmObserver for Events {
  fn on_event(&mut self, event: VmEvent) {
    self.0.push(event);
  }
}

#[test]
fn executes_distinct_named_entries_without_a_synthetic_main() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn init (i64 -> i64)
  local $scratch i64
  const 99
  local.set $scratch
  local.get 0
  const 1
  i.add
  return

fn reload (i64 -> i64)
  local.get 0
  const 2
  i.add
  return"#,
    CalxHostBindings::new(),
  )?;

  assert_eq!(
    vm.run_typed_entry("init", vec![Calx::I64(40)]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(41))
  );
  assert_eq!(
    vm.run_typed_entry("reload", vec![Calx::I64(40)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(42))
  );
  assert_eq!(
    vm.run_typed_entry("init", vec![Calx::I64(0)]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(1))
  );

  let main_error = vm.run_typed(vec![]).expect_err("the compatibility wrapper must still require main");
  assert_eq!(main_error.message, "main function is required");
  Ok(())
}

#[test]
fn named_entries_validate_exact_arguments_and_void_results() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn start (->)
  return

fn scale (i64 -> i64)
  local.get 0
  const 2
  i.mul
  return"#,
    CalxHostBindings::new(),
  )?;

  assert_eq!(
    vm.run_typed_entry("start", vec![]).map_err(|error| error.to_string())?,
    CalxRunResult::Void
  );
  assert_eq!(
    vm.run_typed_entry("scale", vec![Calx::I64(6)]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(12))
  );

  let arity = vm
    .run_typed_entry("scale", vec![])
    .expect_err("the selected entry's arity must be checked");
  assert!(arity.message.contains("entry `scale` expected 1 argument(s), found 0"), "{arity}");
  let value_type = vm
    .run_typed_entry("scale", vec![Calx::Bool(true)])
    .expect_err("the selected entry's parameter type must be checked");
  assert!(
    value_type.message.contains("entry `scale` argument 0 expected I64, found Bool"),
    "{value_type}"
  );
  Ok(())
}

#[test]
fn missing_named_entry_never_falls_back_to_main() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn main (-> i64)
  const 1
  return

fn init (-> i64)
  const 2
  return"#,
    CalxHostBindings::new(),
  )?;

  let error = vm
    .run_typed_entry("missing", vec![])
    .expect_err("an unknown entry must fail before execution");
  assert_eq!(error.message, "typed entry `missing` was not found");
  assert_eq!(error.code(), DiagnosticCode::HostImport);
  assert!(error.snapshot.is_none());
  assert_eq!(
    vm.run_typed(vec![]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(1))
  );
  Ok(())
}

#[test]
fn recursive_named_entry_resets_across_repeated_runs() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn sum (i64 i64 -> i64)
  local.get 0
  const 0
  i.eq
  if (->)
    do
      local.get 1
      return
    do
  local.get 0
  const -1
  i.add
  local.get 1
  local.get 0
  i.add
  return-call sum"#,
    CalxHostBindings::new(),
  )?;

  for size in [100, 0, 10] {
    assert_eq!(
      vm.run_typed_entry("sum", vec![Calx::I64(size), Calx::I64(0)])
        .map_err(|error| error.to_string())?,
      CalxRunResult::Value(Calx::I64(size * (size + 1) / 2))
    );
  }
  Ok(())
}

fn fail_import(_values: &[Calx]) -> Result<Calx, CalxError> {
  Err(CalxError::new_raw("named import failed".to_string()))
}

#[test]
fn named_import_trap_does_not_poison_another_entry() -> Result<(), String> {
  let source = r#"import-fn fail (-> i64)

fn init (-> i64)
  call-import fail
  return

fn reload (-> i64)
  const 7
  return"#;
  let program = parse_program("named-import.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("fail"),
    CalxHostBinding::value(vec![], CalxType::I64, fail_import).map_err(|error| error.to_string())?,
  );
  let mut vm = CalxVM::from_program(program, bindings).map_err(|error| error.to_string())?;

  let error = vm
    .run_typed_entry("init", vec![])
    .expect_err("the selected entry must surface its host import trap");
  assert_eq!(error.code(), DiagnosticCode::HostImport);
  assert_eq!(error.message, "named import failed");
  assert_eq!(
    vm.run_typed_entry("reload", vec![]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(7))
  );
  Ok(())
}

#[test]
fn named_runtime_trap_retains_entry_and_source_span() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn init (->)
  unreachable"#,
    CalxHostBindings::new(),
  )?;

  let error = vm
    .run_typed_entry("init", vec![])
    .expect_err("a trap in a named entry must preserve runtime diagnostics");
  assert_eq!(error.code(), DiagnosticCode::RuntimeTrap);
  assert_eq!(error.top_frame().map(|frame| frame.name.as_ref()), Some("init"));
  assert_eq!(error.source_span().map(|span| span.source.as_ref()), Some("named-entry.cirru"));
  Ok(())
}

#[test]
fn named_trace_shares_selection_reset_and_event_limit() -> Result<(), String> {
  let mut vm = strict_vm(
    r#"fn init (-> i64)
  const 1
  const 2
  i.add
  return

fn reload (-> i64)
  const 9
  return"#,
    CalxHostBindings::new(),
  )?;
  let mut limited = Events::default();
  let error = vm
    .run_traced_entry("init", vec![], 1, &mut limited)
    .expect_err("a named trace must enforce its event limit");
  assert!(matches!(error, CalxTraceError::LimitExceeded { ref function, limit: 1, .. } if function.as_ref() == "init"));
  assert_eq!(limited.0.len(), 1);
  assert_eq!(limited.0[0].function.as_ref(), "init");

  let mut completed = Events::default();
  assert_eq!(
    vm.run_traced_entry("reload", vec![], 8, &mut completed)
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::I64(9))
  );
  assert!(!completed.0.is_empty());
  assert!(completed.0.iter().all(|event| event.function.as_ref() == "reload"));
  assert!(completed.0.iter().all(|event| event
    .source_span
    .as_ref()
    .is_some_and(|span| span.source.as_ref() == "named-entry.cirru")));
  Ok(())
}
