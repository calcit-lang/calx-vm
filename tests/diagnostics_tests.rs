use std::collections::HashMap;
use std::rc::Rc;

use calx_vm::{
  parse_program, validate_program, Calx, CalxError, CalxImportsDict, CalxSyntax, CalxVM, DiagnosticCode, DiagnosticPhase,
  DiagnosticStack, ValidationType,
};

fn source_text<'a>(source: &'a str, span: &calx_vm::SourceSpan) -> &'a str {
  &source[span.start.offset..span.end.offset]
}

#[test]
fn parser_maps_nested_folded_instructions_to_source_tokens() -> Result<(), String> {
  let source = r#"fn main (-> i64)
  i.add (const 1) (i.mul (const 2) (const 3))
  return"#;
  let program = parse_program("nested.cirru", source).map_err(|error| error.to_string())?;
  let main = &program.functions[0];

  assert_eq!(main.syntax.len(), 6);
  assert_eq!(main.source_spans.len(), main.syntax.len());
  let token_names = main
    .source_spans
    .iter()
    .map(|span| source_text(source, span.as_ref().expect("source-aware parsing must attach a span")))
    .collect::<Vec<_>>();
  assert_eq!(token_names, ["const", "const", "const", "i.mul", "i.add", "return"]);
  assert!(matches!(main.syntax[3], CalxSyntax::IntMul));
  assert!(matches!(main.syntax[4], CalxSyntax::IntAdd));
  Ok(())
}

#[test]
fn cirru_parse_errors_keep_the_parser_position() {
  let error = parse_program("invalid.cirru", "fn main ()\n const 1").expect_err("odd indentation must fail");
  let diagnostic = error.diagnostic();

  assert_eq!(diagnostic.code, DiagnosticCode::CirruParse);
  assert_eq!(diagnostic.phase, DiagnosticPhase::Parse);
  let span = diagnostic.span.expect("the Cirru parser reports a position");
  assert_eq!(span.source.as_ref(), "invalid.cirru");
  assert_eq!(span.start.line, 2);
}

#[test]
fn instruction_parse_errors_point_to_the_failing_expression() {
  let source = "fn main ()\n  const 1\n  unknown-op";
  let error = parse_program("unknown.cirru", source).expect_err("unknown instructions must fail");
  let diagnostic = error.diagnostic();

  assert_eq!(diagnostic.code, DiagnosticCode::InstructionParse);
  assert_eq!(diagnostic.phase, DiagnosticPhase::Parse);
  assert_eq!(diagnostic.function, Some("main"));
  assert_eq!(diagnostic.instruction_index, Some(1));
  assert_eq!(source_text(source, diagnostic.span.expect("instruction parse span")), "unknown-op");
}

#[test]
fn validation_errors_expose_span_and_expected_actual_stacks() -> Result<(), String> {
  let source = r#"fn main ()
  const 1.0
  const 2
  i.add
  drop"#;
  let program = parse_program("invalid-type.cirru", source).map_err(|error| error.to_string())?;
  let error = validate_program(&program.functions, &[], &HashMap::new()).expect_err("mixed numeric types must fail validation");
  let diagnostic = error.diagnostic();

  assert_eq!(diagnostic.code, DiagnosticCode::Validation);
  assert_eq!(diagnostic.phase, DiagnosticPhase::Validation);
  assert_eq!(diagnostic.function, Some("main"));
  assert_eq!(diagnostic.instruction_index, Some(2));
  assert_eq!(source_text(source, diagnostic.span.expect("validation span")), "i.add");
  assert_eq!(
    diagnostic.expected_stack,
    Some(DiagnosticStack::ValidationTypes(&[ValidationType::Known(calx_vm::CalxType::I64)]))
  );
  assert_eq!(
    diagnostic.actual_stack,
    Some(DiagnosticStack::ValidationTypes(&[ValidationType::Known(calx_vm::CalxType::F64)]))
  );
  Ok(())
}

#[test]
fn runtime_traps_resolve_the_active_instruction_span() -> Result<(), String> {
  let source = r#"fn main (-> i64)
  const 1
  const 0
  i.div
  return"#;
  let program = parse_program("divide.cirru", source).map_err(|error| error.to_string())?;
  let mut vm = CalxVM::new(program.functions, vec![], HashMap::new());
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  let error = vm.run(vec![]).expect_err("integer division by zero must trap");
  let diagnostic = error.diagnostic();

  assert_eq!(diagnostic.code, DiagnosticCode::RuntimeTrap);
  assert_eq!(diagnostic.phase, DiagnosticPhase::Runtime);
  assert_eq!(diagnostic.function, Some("main"));
  assert_eq!(diagnostic.instruction_index, Some(2));
  assert_eq!(source_text(source, diagnostic.span.expect("runtime span")), "i.div");
  assert!(error.snapshot.is_some());
  Ok(())
}

fn failing_host_import(_args: &Vec<Calx>) -> Result<Calx, CalxError> {
  Err(CalxError::new_raw("host failed".to_string()))
}

#[test]
fn host_import_errors_do_not_fabricate_vm_context() -> Result<(), String> {
  let source = r#"fn main ()
  call-import fail
  drop"#;
  let program = parse_program("host.cirru", source).map_err(|error| error.to_string())?;
  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(Rc::from("fail"), (failing_host_import, 0));
  let mut vm = CalxVM::new(program.functions, vec![], imports);
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  let error = vm.run(vec![]).expect_err("host import must fail");
  let diagnostic = error.diagnostic();

  assert_eq!(diagnostic.code, DiagnosticCode::HostImport);
  assert_eq!(diagnostic.phase, DiagnosticPhase::Host);
  assert!(diagnostic.function.is_none());
  assert!(diagnostic.instruction_index.is_none());
  assert!(diagnostic.span.is_none());
  assert!(error.snapshot.is_none());
  Ok(())
}
