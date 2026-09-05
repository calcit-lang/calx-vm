use std::rc::Rc;

use calx_vm::{
  parse_program, Calx, CalxBuildErrorKind, CalxError, CalxHostBinding, CalxHostBindings, CalxProgram, CalxRunResult, CalxSyntax,
  CalxType, CalxVM, DiagnosticCode, FunctionBuilder, ProgramBuilder, ValidatedProgram,
};

#[test]
fn strict_declarations_reject_untyped_lists_at_every_boundary() {
  for source in [
    "fn main (list ->)\n  drop",
    "fn main (-> list)\n  unreachable",
    "fn main ()\n  local $x list\n  return",
    "global $x (const list) nil\nfn main ()\n  return",
    "import-fn host (list ->)\nfn main ()\n  return",
    "import-fn host (-> list)\nfn main ()\n  return",
  ] {
    let parsed = parse_program("list-boundary.cirru", source).unwrap();
    let error = parsed
      .into_program()
      .expect_err("an outer List tag does not prove its element types");
    assert!(error.message.contains("List"), "{error}");
    assert!(error.message.contains("element"), "{error}");
  }
}

#[test]
fn strict_control_signatures_reject_legacy_types_even_in_dead_code() {
  for value_type in ["nil", "list", "link"] {
    for body in [
      format!("block ({value_type} ->)\n    unreachable"),
      format!("block (-> {value_type})\n    unreachable"),
      format!("loop ({value_type} ->)\n    unreachable"),
      format!("loop (-> {value_type})\n    unreachable"),
      format!("if (-> {value_type})\n    do\n      unreachable\n    do\n      unreachable"),
    ] {
      let source = format!("fn main ()\n  unreachable\n  {body}\n  drop");
      let parsed = parse_program("control-domain.cirru", &source).unwrap();
      let error = parsed
        .into_program()
        .expect_err("unreachable control signatures still belong to the strict profile");
      assert!(error.message.contains("strict control"), "{error}");
      assert_eq!(error.function.as_deref(), Some("main"));
      assert!(error.span.is_some(), "control diagnostics must retain source origin: {error}");
    }
  }
}

#[test]
fn parsed_nil_constants_cannot_enter_strict_execution() {
  for prefix in ["", "  unreachable\n"] {
    let source = format!("fn main ()\n{prefix}  const nil\n  drop");
    let parsed = parse_program("nil-constant.cirru", &source).unwrap();
    let error = parsed
      .into_program()
      .expect_err("even dropped/dead Nil constants are not strict values");
    assert!(error.message.contains("strict constant cannot use Nil"), "{error}");
    assert_eq!(error.diagnostic().code, DiagnosticCode::Validation);
    let span = error.span.as_ref().expect("constant origin");
    assert_eq!(&source[span.start.offset..span.end.offset], "const");
  }
}

#[test]
fn direct_rust_program_rejects_list_constants_without_inspecting_elements() {
  for value in [Calx::List(vec![]), Calx::List(vec![Calx::Nil]), Calx::List(vec![Calx::I64(1)])] {
    let mut parsed = parse_program("rust-ir.cirru", "fn main ()\n  const 1\n  drop").unwrap();
    Rc::make_mut(&mut parsed.functions[0].syntax)[0] = CalxSyntax::Const(value);
    let error = CalxProgram::try_new(parsed.functions, vec![], vec![]).expect_err("List must be rejected even if empty/homogeneous");
    assert!(error.message.contains("strict constant cannot use List"), "{error}");
  }
}

#[test]
fn builder_rejects_legacy_constants_atomically() {
  let mut main = FunctionBuilder::synthetic("main", vec![CalxType::I64], "generated:strict-domain").unwrap();
  for value in [Calx::Nil, Calx::List(vec![Calx::Nil])] {
    let error = main.body().constant(value).expect_err("builder must reject before emission");
    assert_eq!(error.kind, CalxBuildErrorKind::InvalidType);
    assert!(error.span.is_some());
  }
  main.body().constant(Calx::I64(42)).unwrap().return_().unwrap();
  let mut builder = ProgramBuilder::new();
  builder.function(main).unwrap();
  let program = builder.build().unwrap();
  assert_eq!(
    program.functions()[0].syntax.len(),
    2,
    "failed constants must not leave instructions behind"
  );
  let mut vm = CalxVM::from_program(program, CalxHostBindings::new()).unwrap();
  assert_eq!(vm.run_typed(vec![]).unwrap(), CalxRunResult::Value(Calx::I64(42)));
}

fn host_value(_args: &[Calx]) -> Result<Calx, CalxError> {
  Ok(Calx::List(vec![Calx::Nil]))
}

fn host_void(_args: &[Calx]) -> Result<(), CalxError> {
  Ok(())
}

#[test]
fn host_list_contracts_are_rejected_and_lying_results_still_trap() {
  assert!(CalxHostBinding::void(vec![CalxType::List], host_void).is_err());
  assert!(CalxHostBinding::value(vec![CalxType::List], CalxType::I64, host_value).is_err());
  assert!(CalxHostBinding::value(vec![], CalxType::List, host_value).is_err());
  let program = parse_program("host.cirru", "import-fn host (-> i64)\nfn main (-> i64)\n  call-import host")
    .unwrap()
    .into_program()
    .unwrap();
  let mut bindings = CalxHostBindings::new();
  bindings.insert(Rc::from("host"), CalxHostBinding::value(vec![], CalxType::I64, host_value).unwrap());
  let mut vm = CalxVM::from_program(program, bindings).unwrap();
  assert!(vm.run_typed(vec![]).is_err(), "declared host types cannot prove callback values");
}

#[test]
fn concrete_entry_values_still_validate_and_execute() {
  for (name, value) in [
    ("bool", Calx::Bool(true)),
    ("i64", Calx::I64(42)),
    ("f64", Calx::F64(1.5)),
    ("str", Calx::Str(Rc::from("typed"))),
    ("f64-buffer", Calx::f64_buffer_copy_from_slice(&[1.0, 2.0])),
  ] {
    let source = format!("fn main (($x {name}) -> {name})\n  local.get $x");
    let program = parse_program("identity.cirru", &source).unwrap().into_program().unwrap();
    let validated = ValidatedProgram::try_from_program(program).unwrap();
    let mut vm = CalxVM::from_validated_program(validated, CalxHostBindings::new()).unwrap();
    assert_eq!(vm.run_typed(vec![value.clone()]).unwrap(), CalxRunResult::Value(value));
  }
}

#[test]
fn explicit_legacy_nil_and_list_values_remain_supported() {
  for value in [Calx::Nil, Calx::List(vec![Calx::I64(1), Calx::Nil])] {
    let value_type = if matches!(value, Calx::Nil) { "nil" } else { "list" };
    let source = format!("fn main (($x {value_type}) -> {value_type})\n  local.get $x");
    let parsed = parse_program("legacy.cirru", &source).unwrap();
    let mut vm = CalxVM::new(parsed.functions, vec![], Default::default());
    vm.preprocess(false).unwrap();
    vm.setup_top_frame().unwrap();
    assert_eq!(vm.run(vec![value.clone()]).unwrap(), value);
  }
}
