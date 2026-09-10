use std::rc::Rc;

use calx_vm::{
  parse_program, Calx, CalxHostBinding, CalxHostBindings, CalxMutability, CalxRunResult, CalxType, CalxVM, FunctionBuilder,
  ProgramBuilder,
};

fn echo_tag(values: &[Calx]) -> Result<Calx, calx_vm::CalxError> {
  let [Calx::Tag(value)] = values else {
    return Ok(Calx::Str(Rc::from("wrong-argument")));
  };
  Ok(Calx::Tag(value.clone()))
}

fn wrong_tag_result(_values: &[Calx]) -> Result<Calx, calx_vm::CalxError> {
  Ok(Calx::Str(Rc::from("not-a-tag")))
}

#[test]
fn parses_and_renders_tag_and_string_without_aliasing() -> Result<(), String> {
  let string = "|ready".parse::<Calx>()?;
  let tag = ":ready".parse::<Calx>()?;

  assert_eq!(string, Calx::Str(Rc::from("ready")));
  assert_eq!(tag, Calx::Tag(Rc::from("ready")));
  assert_eq!(string.value_type(), CalxType::Str);
  assert_eq!(tag.value_type(), CalxType::Tag);
  assert_eq!(string.to_string(), "|ready");
  assert_eq!(tag.to_string(), ":ready");
  assert_eq!(string.to_string().parse::<Calx>()?, string);
  assert_eq!(tag.to_string().parse::<Calx>()?, tag);
  assert_eq!(format!("{string:?}"), "Str(\"ready\")");
  assert_eq!(format!("{tag:?}"), "Tag(\"ready\")");
  assert_eq!("tag".parse::<CalxType>()?, CalxType::Tag);
  Ok(())
}

#[test]
fn encoded_tag_type_is_appended_without_renumbering_existing_types() -> Result<(), String> {
  let config = bincode::config::standard();
  let link = bincode::encode_to_vec(CalxType::Link, config).map_err(|error| error.to_string())?;
  let tag = bincode::encode_to_vec(CalxType::Tag, config).map_err(|error| error.to_string())?;
  assert_eq!(link, vec![7], "the previous final discriminant must remain stable");
  assert_eq!(tag, vec![8], "Tag must be appended after every existing type");
  let (decoded, consumed) = bincode::decode_from_slice::<CalxType, _>(&tag, config).map_err(|error| error.to_string())?;
  assert_eq!(decoded, CalxType::Tag);
  assert_eq!(consumed, tag.len());
  Ok(())
}

#[test]
fn strict_tag_values_cross_entries_locals_globals_and_control() -> Result<(), String> {
  let source = r#"global $fallback (const tag) :cold

fn init (($input tag) -> tag)
  local $held tag
  local.get $input
  local.set $held
  const true
  if (-> tag)
    do
      local.get $held
    do
      global.get $fallback
  return

fn reload (($input tag) -> tag)
  local.get $input
  block (tag -> tag)
  return

fn looped (($input tag) -> tag)
  local.get $input
  loop (tag -> tag)
  return"#;
  let program = parse_program("tag-entry.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  let mut vm = CalxVM::from_program(program, CalxHostBindings::new()).map_err(|error| error.to_string())?;

  let ready = Calx::Tag(Rc::from("ready"));
  assert_eq!(
    vm.run_typed_entry("init", vec![ready.clone()]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(ready.clone())
  );
  assert_eq!(
    vm.run_typed_entry("reload", vec![ready.clone()])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(ready.clone())
  );
  assert_eq!(
    vm.run_typed_entry("looped", vec![ready.clone()])
      .map_err(|error| error.to_string())?,
    CalxRunResult::Value(ready)
  );

  let mismatch = vm
    .run_typed_entry("init", vec![Calx::Str(Rc::from("ready"))])
    .expect_err("Str must not satisfy a Tag parameter");
  assert!(mismatch.message.contains("expected Tag, found Str"), "{mismatch}");
  Ok(())
}

#[test]
fn strict_tag_host_imports_preserve_argument_and_result_identity() -> Result<(), String> {
  let source = r#"import-fn echo-tag (tag -> tag)

fn main (($input tag) -> tag)
  local.get $input
  call-import echo-tag
  return"#;
  let program = parse_program("tag-import.cirru", source)
    .map_err(|error| error.to_string())?
    .into_program()
    .map_err(|error| error.to_string())?;
  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("echo-tag"),
    CalxHostBinding::value(vec![CalxType::Tag], CalxType::Tag, echo_tag).map_err(|error| error.to_string())?,
  );
  let mut vm = CalxVM::from_program(program.clone(), bindings).map_err(|error| error.to_string())?;
  assert_eq!(
    vm.run_typed(vec![Calx::Tag(Rc::from("ok"))]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Tag(Rc::from("ok")))
  );

  let mut wrong_bindings = CalxHostBindings::new();
  wrong_bindings.insert(
    Rc::from("echo-tag"),
    CalxHostBinding::value(vec![CalxType::Tag], CalxType::Tag, wrong_tag_result).map_err(|error| error.to_string())?,
  );
  let mut wrong_vm = CalxVM::from_program(program, wrong_bindings).map_err(|error| error.to_string())?;
  let error = wrong_vm
    .run_typed(vec![Calx::Tag(Rc::from("ok"))])
    .expect_err("a host String must not satisfy a declared Tag result");
  assert!(error.message.contains("expected Tag, found Str"), "{error}");
  Ok(())
}

#[test]
fn builder_admits_tag_declarations_constants_and_control() -> Result<(), String> {
  let mut builder = ProgramBuilder::new();
  let fallback = builder
    .global("$fallback", CalxType::Tag, CalxMutability::Const, Calx::Tag(Rc::from("cold")))
    .map_err(|error| error.to_string())?;
  let echo = builder
    .import("echo-tag", vec![CalxType::Tag], Some(CalxType::Tag))
    .map_err(|error| error.to_string())?;

  let mut main = FunctionBuilder::new("main", vec![CalxType::Tag]).map_err(|error| error.to_string())?;
  let input = main.parameter("$input", CalxType::Tag).map_err(|error| error.to_string())?;
  let held = main.local("$held", CalxType::Tag).map_err(|error| error.to_string())?;
  main
    .body()
    .local_get(&input)
    .and_then(|body| body.call_import(&echo))
    .and_then(|body| body.local_set(&held))
    .and_then(|body| body.constant(Calx::Bool(true)))
    .map_err(|error| error.to_string())?;
  main
    .body()
    .if_else(
      vec![CalxType::Tag],
      |body| {
        body.local_get(&held)?;
        Ok(())
      },
      |body| {
        body.global_get(&fallback)?;
        Ok(())
      },
    )
    .map_err(|error| error.to_string())?;
  main.body().return_().map_err(|error| error.to_string())?;
  builder.function(main).map_err(|error| error.to_string())?;

  let mut bindings = CalxHostBindings::new();
  bindings.insert(
    Rc::from("echo-tag"),
    CalxHostBinding::value(vec![CalxType::Tag], CalxType::Tag, echo_tag).map_err(|error| error.to_string())?,
  );
  let mut vm =
    CalxVM::from_program(builder.build().map_err(|error| error.to_string())?, bindings).map_err(|error| error.to_string())?;
  assert_eq!(
    vm.run_typed(vec![Calx::Tag(Rc::from("warm"))]).map_err(|error| error.to_string())?,
    CalxRunResult::Value(Calx::Tag(Rc::from("warm")))
  );
  Ok(())
}

#[test]
fn validator_rejects_string_where_tag_is_required_before_execution() -> Result<(), String> {
  let program = parse_program(
    "tag-mismatch.cirru",
    r#"fn main (-> tag)
  const |ready
  return"#,
  )
  .map_err(|error| error.to_string())?
  .into_program()
  .map_err(|error| error.to_string())?;
  let error = CalxVM::from_program(program, CalxHostBindings::new()).expect_err("Str must not validate as Tag");
  assert!(error.message.contains("expected Tag, found Str"), "{error}");
  assert_eq!(error.function.as_deref(), Some("main"));
  assert!(error.span.is_some());
  Ok(())
}
