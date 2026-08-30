use calx_vm::{parse_program, Calx, CalxBoundaryType, CalxError, CalxHostBinding, CalxMutability, CalxSyntax, CalxType};

fn source_text<'a>(source: &'a str, span: &calx_vm::SourceSpan) -> &'a str {
  &source[span.start.offset..span.end.offset]
}

#[test]
fn parses_interleaved_typed_module_declarations_and_forward_globals() -> Result<(), String> {
  let source = r#"import-fn notify (i64 ->)

fn main (-> i64)
  local $value i64
  global.get $later
  local.set $value
  local.get $value
  return

global $later (mut i64) 7
global $build-id (const str) |dev"#;

  let parsed = parse_program("typed.cirru", source).map_err(|error| error.to_string())?;
  assert_eq!(parsed.nodes.len(), 1, "top-level declarations must not enter function nodes");
  assert_eq!(parsed.functions.len(), 1);
  assert_eq!(parsed.globals.len(), 2);
  assert_eq!(parsed.imports.len(), 1);
  assert_eq!(parsed.dynamic_boundary_count(), 0);

  let main = &parsed.functions[0];
  assert_eq!(main.locals.len(), 1);
  assert_eq!(main.locals[0].name.as_ref(), "$value");
  assert_eq!(main.locals[0].value_type, CalxBoundaryType::Known(CalxType::I64));
  assert_eq!(main.local_names.as_ref(), &["$value"]);
  assert!(main.syntax.iter().all(|syntax| !matches!(syntax, CalxSyntax::LocalNew)));
  assert!(matches!(main.syntax[0], CalxSyntax::GlobalGet(0)));
  assert_eq!(main.source_spans.len(), main.syntax.len());
  assert_eq!(
    source_text(
      source,
      main.locals[0].span.as_ref().expect("typed local must retain its source span")
    ),
    "local $value i64"
  );

  assert_eq!(parsed.globals[0].name.as_ref(), "$later");
  assert_eq!(parsed.globals[0].mutability, CalxMutability::Mut);
  assert_eq!(parsed.globals[0].initial, Calx::I64(7));
  assert_eq!(parsed.imports[0].result, None);

  let strict = parsed.into_program().map_err(|error| error.to_string())?;
  assert_eq!(strict.functions().len(), 1);
  assert_eq!(strict.globals().len(), 2);
  assert_eq!(strict.imports().len(), 1);
  Ok(())
}

#[test]
fn parses_zero_and_single_result_imports() -> Result<(), String> {
  let source = r#"import-fn notify (i64 i64 ->)
import-fn add2 (i64 i64 -> i64)

fn main (-> i64)
  const 2
  const 3
  call-import notify
  const 20
  const 22
  call-import add2
  return"#;
  let parsed = parse_program("imports.cirru", source).map_err(|error| error.to_string())?;
  assert_eq!(parsed.imports[0].result, None);
  assert_eq!(parsed.imports[1].result, Some(CalxBoundaryType::Known(CalxType::I64)));
  parsed.into_program().map_err(|error| error.to_string())?;
  Ok(())
}

#[test]
fn legacy_local_is_observable_and_rejected_by_strict_conversion() -> Result<(), String> {
  let source = r#"fn main ()
  local.new $value
  const 1
  local.set $value
  return"#;
  let parsed = parse_program("legacy.cirru", source).map_err(|error| error.to_string())?;
  assert_eq!(parsed.dynamic_boundary_count(), 1);
  assert_eq!(parsed.functions[0].locals[0].value_type, CalxBoundaryType::Dynamic);
  assert!(matches!(parsed.functions[0].syntax[0], CalxSyntax::LocalNew));

  let error = parsed.into_program().expect_err("strict conversion must reject a Dynamic local");
  assert!(error.message.contains("cannot use Dynamic"), "{error}");
  assert_eq!(error.function.as_deref(), Some("main"));
  assert!(error.span.is_some());
  Ok(())
}

#[test]
fn legacy_global_and_undeclared_import_are_counted() -> Result<(), String> {
  let source = r#"fn main ()
  global.new
  call-import legacy-log
  drop
  return"#;
  let parsed = parse_program("legacy-boundaries.cirru", source).map_err(|error| error.to_string())?;
  assert_eq!(parsed.dynamic_boundary_count(), 2);
  let error = parsed
    .into_program()
    .expect_err("legacy boundaries must not enter a strict program");
  assert!(
    error.message.contains("local.new/global.new") || error.message.contains("undeclared import"),
    "{error}"
  );
  Ok(())
}

#[test]
fn strict_conversion_rejects_nil_boundaries() {
  for (source, boundary) in [
    ("fn main (nil ->)\n  nop", "function"),
    ("fn main ()\n  local $value nil\n  return", "local"),
    ("global $value (mut nil) nil\nfn main ()\n  return", "global"),
    ("import-fn read (nil ->)\nfn main ()\n  return", "import"),
  ] {
    let parsed = parse_program("nil.cirru", source).expect("Nil remains parseable as explicit legacy data");
    let error = parsed.into_program().expect_err("Nil must not enter a strict boundary");
    assert!(error.message.contains("Nil"), "{boundary}: {error}");
  }
}

#[test]
fn strict_conversion_rejects_initializer_mismatch_and_undeclared_import() {
  let mismatch = parse_program("global.cirru", "global $count (mut i64) true\nfn main ()\n  return")
    .expect("the parser preserves the declared contract");
  let error = mismatch
    .into_program()
    .expect_err("initializer type must be checked before runtime");
  assert!(error.message.contains("initializer type mismatch"), "{error}");

  let undeclared = parse_program("import.cirru", "fn main ()\n  call-import missing\n  drop\n  return")
    .expect("call-import syntax is valid before strict module conversion");
  let error = undeclared.into_program().expect_err("strict programs require import declarations");
  assert!(error.message.contains("undeclared import `missing`"), "{error}");
}

#[test]
fn parser_rejects_ambiguous_or_late_declarations() {
  for (source, expected) in [
    (
      "fn main (($value i64) ->)\n  local $value i64\n  return",
      "duplicate local declaration `$value`",
    ),
    (
      "fn main ()\n  nop\n  local $value i64",
      "local declarations must appear before the first executable instruction",
    ),
    ("fn main ()\n  local.get $missing\n  drop", "unknown local `$missing`"),
    (
      "global $value (mut i64) 0\nglobal $value (mut i64) 1\nfn main ()\n  return",
      "duplicate global declaration `$value`",
    ),
    (
      "import-fn log (i64 ->)\nimport-fn log (i64 ->)\nfn main ()\n  return",
      "duplicate import declaration `log`",
    ),
    ("import-fn pair (-> i64 i64)\nfn main ()\n  return", "supports zero or one result"),
    (
      "import-fn malformed (i64 -> -> i64)\nfn main ()\n  return",
      "requires exactly one `->`",
    ),
    ("import-fn named (($value i64) ->)\nfn main ()\n  return", "must be plain tokens"),
    ("import-fn loose (dynamic ->)\nfn main ()\n  return", "unknown type: dynamic"),
  ] {
    let error = parse_program("invalid-module.cirru", source).expect_err("invalid declaration must fail parsing");
    assert!(error.message.contains(expected), "expected {expected:?}, got {error}");
    assert!(error.span.is_some());
  }
}

fn host_void(_args: &[Calx]) -> Result<(), CalxError> {
  Ok(())
}

fn host_value(_args: &[Calx]) -> Result<Calx, CalxError> {
  Ok(Calx::I64(1))
}

#[test]
fn typed_host_binding_constructors_exclude_nil() -> Result<(), String> {
  let void = CalxHostBinding::void(vec![CalxType::I64], host_void).map_err(|error| error.to_string())?;
  assert_eq!(void.result(), None);
  let value = CalxHostBinding::value(vec![CalxType::I64], CalxType::I64, host_value).map_err(|error| error.to_string())?;
  assert_eq!(value.result(), Some(CalxType::I64));

  let error = CalxHostBinding::void(vec![CalxType::Nil], host_void).expect_err("Nil parameter must be rejected");
  assert!(error.message.contains("Nil"), "{error}");
  let error = CalxHostBinding::value(vec![], CalxType::Nil, host_value).expect_err("Nil result must be rejected in typed host ABI");
  assert!(error.message.contains("Nil"), "{error}");
  Ok(())
}
