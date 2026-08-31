use std::rc::Rc;

use calx_vm::{
  parse_program, Calx, CalxBuildErrorKind, CalxFunc, CalxGlobalDecl, CalxHostBinding, CalxHostBindings, CalxMutability, CalxProgram,
  CalxRunResult, CalxSyntax, CalxType, CalxVM, FunctionBuilder, ProgramBuilder, ValidatedProgram,
};

type TestResult<T = ()> = Result<T, String>;

fn strict_vm(source: &str, bindings: CalxHostBindings) -> TestResult<CalxVM> {
  let program = parse_program("f64-buffer-test.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  CalxVM::from_program(program, bindings).map_err(|error| error.to_string())
}

fn return_buffer(_: &[Calx]) -> Result<Calx, calx_vm::CalxError> {
  Ok(Calx::f64_buffer_adopt(vec![2.0, 4.0, 8.0]))
}

fn return_wrong_buffer(_: &[Calx]) -> Result<Calx, calx_vm::CalxError> {
  Ok(Calx::List(vec![Calx::F64(2.0)]))
}

fn first_buffer_value(values: &[Calx]) -> Result<Calx, calx_vm::CalxError> {
  let [Calx::F64Buffer(buffer)] = values else {
    return Err(calx_vm::CalxError::new_raw(format!("expected one F64Buffer, got {values:?}")));
  };
  Ok(Calx::F64(buffer.first().copied().unwrap_or(0.0)))
}

#[test]
fn f64_buffer_constructors_share_adopt_copy_and_hide_elements_in_diagnostics() -> TestResult {
  assert_eq!("f64-buffer".parse::<CalxType>()?, CalxType::F64Buffer);

  let backing: Rc<[f64]> = Rc::from(vec![1.0, 2.0, 3.0].into_boxed_slice());
  let shared = Calx::f64_buffer_share(backing.clone());
  let cloned = shared.clone();
  let (Calx::F64Buffer(shared_backing), Calx::F64Buffer(cloned_backing)) = (&shared, &cloned) else {
    return Err("shared values must remain F64Buffer".to_string());
  };
  assert!(Rc::ptr_eq(shared_backing, &backing));
  assert!(Rc::ptr_eq(shared_backing, cloned_backing));

  let adopted = Calx::f64_buffer_adopt(vec![4.0, 5.0]);
  let copied = Calx::f64_buffer_copy_from_slice(&[6.0, 7.0]);
  assert_eq!(adopted.as_f64_buffer(), Some([4.0, 5.0].as_slice()));
  assert_eq!(copied.as_f64_buffer(), Some([6.0, 7.0].as_slice()));
  assert_eq!(format!("{shared}"), "#<f64-buffer len=3>");
  assert_eq!(format!("{shared:?}"), "F64Buffer { len: 3 }");
  assert!(!format!("{shared:?}").contains("1.0"));
  Ok(())
}

#[test]
fn parser_validator_lowering_and_runtime_execute_len_conversion_and_get() -> TestResult {
  let source = r#"fn main (f64-buffer f64 -> f64)
  local.get 0
  local.get 1
  f64.to-i64-index
  f64-buffer.get
  return"#;
  let parsed = parse_program("f64-buffer-source.cirru", source).map_err(|error| error.to_string())?;
  assert_eq!(parsed.functions[0].params_types.as_ref(), &[CalxType::F64Buffer, CalxType::F64]);
  assert!(matches!(parsed.functions[0].syntax[2], CalxSyntax::F64ToI64Index));
  assert!(matches!(parsed.functions[0].syntax[3], CalxSyntax::F64BufferGet));

  let mut vm = strict_vm(source, CalxHostBindings::new())?;
  let result = vm
    .run_typed(vec![Calx::f64_buffer_adopt(vec![1.5, 2.5, 3.5]), Calx::F64(1.0)])
    .map_err(|error| error.to_string())?;
  assert_eq!(result, CalxRunResult::Value(Calx::F64(2.5)));

  let mut len_vm = strict_vm(
    r#"fn main (f64-buffer -> i64)
  local.get 0
  f64-buffer.len
  return"#,
    CalxHostBindings::new(),
  )?;
  for (values, expected) in [(vec![], 0), (vec![1.0], 1), (vec![1.0, 2.0, 3.0], 3)] {
    assert_eq!(
      len_vm
        .run_typed(vec![Calx::f64_buffer_adopt(values)])
        .map_err(|error| error.to_string())?,
      CalxRunResult::Value(Calx::I64(expected))
    );
  }
  Ok(())
}

#[test]
fn builder_supports_buffer_boundaries_locals_blocks_loops_and_helpers() -> TestResult {
  let mut program = ProgramBuilder::new();

  let mut loop_len =
    FunctionBuilder::synthetic("loop-len", vec![CalxType::I64], "generated:test/loop-len").map_err(|error| error.to_string())?;
  let loop_buffer = loop_len
    .parameter("$buffer", CalxType::F64Buffer)
    .map_err(|error| error.to_string())?;
  loop_len
    .body()
    .local_get(&loop_buffer)
    .map_err(|error| error.to_string())?
    .loop_(vec![CalxType::F64Buffer], vec![CalxType::F64Buffer], |_| Ok(()))
    .map_err(|error| error.to_string())?
    .f64_buffer_len()
    .map_err(|error| error.to_string())?
    .return_()
    .map_err(|error| error.to_string())?;
  program.function(loop_len).map_err(|error| error.to_string())?;

  let mut main = FunctionBuilder::synthetic("main", vec![CalxType::F64], "generated:test/main").map_err(|error| error.to_string())?;
  let buffer = main.parameter("$buffer", CalxType::F64Buffer).map_err(|error| error.to_string())?;
  let index = main.parameter("$index", CalxType::F64).map_err(|error| error.to_string())?;
  let held = main.local("$held", CalxType::F64Buffer).map_err(|error| error.to_string())?;
  main
    .body()
    .local_get(&buffer)
    .map_err(|error| error.to_string())?
    .block(vec![CalxType::F64Buffer], vec![CalxType::F64Buffer], |_| Ok(()))
    .map_err(|error| error.to_string())?
    .local_set(&held)
    .map_err(|error| error.to_string())?
    .local_get(&held)
    .map_err(|error| error.to_string())?
    .local_get(&index)
    .map_err(|error| error.to_string())?
    .f64_to_i64_index()
    .map_err(|error| error.to_string())?
    .f64_buffer_get()
    .map_err(|error| error.to_string())?
    .return_()
    .map_err(|error| error.to_string())?;
  program.function(main).map_err(|error| error.to_string())?;

  let mut vm = CalxVM::from_program(program.build().map_err(|error| error.to_string())?, CalxHostBindings::new())
    .map_err(|error| error.to_string())?;
  assert_eq!(
    vm.run_typed(vec![Calx::f64_buffer_adopt(vec![10.0, 20.0]), Calx::F64(1.0)])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::F64(20.0))
  );
  Ok(())
}

#[test]
fn globals_constants_wrong_stack_types_and_truthiness_are_rejected() -> TestResult {
  let mut builder = ProgramBuilder::new();
  let global_error = builder
    .global(
      "$buffer",
      CalxType::F64Buffer,
      CalxMutability::Const,
      Calx::f64_buffer_adopt(vec![1.0]),
    )
    .expect_err("F64Buffer globals must be rejected by the builder");
  assert_eq!(global_error.kind, CalxBuildErrorKind::InvalidType);

  let main = CalxFunc::new("main", vec![], vec![], vec![CalxSyntax::Return]);
  let global = CalxGlobalDecl::new(
    "$buffer",
    CalxType::F64Buffer,
    CalxMutability::Const,
    Calx::f64_buffer_adopt(vec![1.0]),
  );
  let global_error = CalxProgram::try_new(vec![main], vec![global], vec![]).expect_err("public IR must reject buffer globals");
  assert!(global_error.message.contains("cannot use F64Buffer"), "{global_error}");

  let constant_function = CalxFunc::new(
    "main",
    vec![],
    vec![CalxType::F64Buffer],
    vec![CalxSyntax::Const(Calx::f64_buffer_adopt(vec![1.0])), CalxSyntax::Return],
  );
  let constant_program = CalxProgram::try_new(vec![constant_function], vec![], vec![]).map_err(|error| error.to_string())?;
  let error = ValidatedProgram::try_from_program(constant_program).expect_err("public IR must reject buffer constants");
  assert!(error.message.contains("constants are not supported"), "{error}");

  for source in [
    r#"fn main (list -> i64)
  local.get 0
  f64-buffer.len
  return"#,
    r#"fn main (f64-buffer -> i64)
  local.get 0
  if (-> i64)
    do $ const 1
    do $ const 2
  return"#,
  ] {
    let program = parse_program("invalid-buffer.cirru", source)
      .map_err(|error| error.to_string())?
      .into_program()
      .map_err(|error| error.to_string())?;
    let error = ValidatedProgram::try_from_program(program).expect_err("invalid F64Buffer use must fail validation");
    assert!(
      error.message.contains("expected F64Buffer") || error.message.contains("does not participate in truthiness"),
      "{error}"
    );
  }
  Ok(())
}

#[test]
fn checked_index_conversion_accepts_only_the_frozen_half_open_domain() -> TestResult {
  let mut vm = strict_vm(
    r#"fn main (f64 -> i64)
  local.get 0
  f64.to-i64-index
  return"#,
    CalxHostBindings::new(),
  )?;

  for (value, expected) in [(-0.0, 0), (0.0, 0), (42.0, 42), (9_007_199_254_740_992.0, 9_007_199_254_740_992)] {
    assert_eq!(
      vm.run_typed(vec![Calx::F64(value)]).map_err(|error| error.to_string())?,
      CalxRunResult::Value(Calx::I64(expected))
    );
  }

  for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.5, -1.0, 9_223_372_036_854_775_808.0] {
    let error = vm
      .run_typed(vec![Calx::F64(value)])
      .expect_err("invalid index conversion must trap");
    assert_eq!(error.code(), calx_vm::DiagnosticCode::RuntimeTrap);
    assert!(error.message.contains("f64.to-i64-index invalid value"), "{error}");
  }
  Ok(())
}

#[test]
fn buffer_get_reports_negative_equal_and_greater_bounds_without_nil() -> TestResult {
  let mut vm = strict_vm(
    r#"fn main (f64-buffer i64 -> f64)
  local.get 0
  local.get 1
  f64-buffer.get
  return"#,
    CalxHostBindings::new(),
  )?;

  for index in [-1, 2, 3] {
    let error = vm
      .run_typed(vec![Calx::f64_buffer_adopt(vec![1.0, 2.0]), Calx::I64(index)])
      .expect_err("out-of-bounds access must trap");
    assert_eq!(error.code(), calx_vm::DiagnosticCode::RuntimeTrap);
    assert!(error.message.contains("f64-buffer.get"), "{error}");
    assert!(error.message.contains(&format!("index {index}")), "{error}");
    assert!(error.message.contains("length 2"), "{error}");
  }
  Ok(())
}

#[test]
fn typed_imports_recheck_buffer_signatures_and_runtime_variants() -> TestResult {
  let source = r#"import-fn source-buffer (-> f64-buffer)
import-fn first-value (f64-buffer -> f64)

fn main (-> f64)
  call-import source-buffer
  call-import first-value
  return"#;

  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("source-buffer"),
    CalxHostBinding::value(vec![], CalxType::F64Buffer, return_buffer).map_err(|error| error.to_string())?,
  );
  bindings.insert(
    Rc::from("first-value"),
    CalxHostBinding::value(vec![CalxType::F64Buffer], CalxType::F64, first_buffer_value).map_err(|error| error.to_string())?,
  );
  let mut vm = strict_vm(source, bindings)?;
  assert_eq!(
    vm.run_typed(vec![]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::F64(2.0))
  );

  let mut wrong_bindings = CalxHostBindings::new();
  wrong_bindings.insert(
    Rc::from("source-buffer"),
    CalxHostBinding::value(vec![], CalxType::F64Buffer, return_wrong_buffer).map_err(|error| error.to_string())?,
  );
  wrong_bindings.insert(
    Rc::from("first-value"),
    CalxHostBinding::value(vec![CalxType::F64Buffer], CalxType::F64, first_buffer_value).map_err(|error| error.to_string())?,
  );
  let mut wrong_vm = strict_vm(source, wrong_bindings)?;
  let error = wrong_vm
    .run_typed(vec![])
    .expect_err("wrong host result variant must fail at the boundary");
  assert!(error.message.contains("result expected F64Buffer, found List"), "{error}");

  let mut signature_bindings = CalxHostBindings::new();
  signature_bindings.insert(
    Rc::from("source-buffer"),
    CalxHostBinding::value(vec![], CalxType::List, return_wrong_buffer).map_err(|error| error.to_string())?,
  );
  signature_bindings.insert(
    Rc::from("first-value"),
    CalxHostBinding::value(vec![CalxType::F64Buffer], CalxType::F64, first_buffer_value).map_err(|error| error.to_string())?,
  );
  let error = strict_vm(source, signature_bindings).expect_err("wrong declared host signature must fail before execution");
  assert!(error.contains("signature mismatch"), "{error}");
  Ok(())
}

#[test]
fn malformed_public_instructions_return_errors_instead_of_panicking() -> TestResult {
  let cases = [
    (
      vec![Calx::List(vec![])],
      vec![calx_vm::CalxInstr::LocalGet(0), calx_vm::CalxInstr::F64BufferLen],
      "f64-buffer.len expected F64Buffer",
    ),
    (
      vec![Calx::Bool(true)],
      vec![calx_vm::CalxInstr::LocalGet(0), calx_vm::CalxInstr::F64ToI64Index],
      "f64.to-i64-index expected F64",
    ),
    (
      vec![Calx::List(vec![]), Calx::I64(0)],
      vec![
        calx_vm::CalxInstr::LocalGet(0),
        calx_vm::CalxInstr::LocalGet(1),
        calx_vm::CalxInstr::F64BufferGet,
      ],
      "f64-buffer.get expected F64Buffer",
    ),
    (
      vec![Calx::f64_buffer_adopt(vec![1.0])],
      vec![
        calx_vm::CalxInstr::LocalGet(0),
        calx_vm::CalxInstr::Assert(Rc::from("buffer condition")),
      ],
      "F64Buffer does not participate in truthiness",
    ),
  ];

  for (args, instrs, expected) in cases {
    let params = args.iter().map(Calx::value_type).collect();
    let main = CalxFunc::new("main", params, vec![], vec![]).with_instrs(instrs);
    let mut vm = CalxVM::new(vec![main], vec![], Default::default());
    vm.setup_top_frame()?;
    let error = vm.run(args).expect_err("malformed public instruction must return CalxError");
    assert!(error.message.contains(expected), "{error}");
  }
  Ok(())
}
