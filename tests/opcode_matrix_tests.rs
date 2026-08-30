use std::collections::HashMap;
use std::rc::Rc;

use calx_vm::{parse_function, Calx, CalxError, CalxFunc, CalxImportsDict, CalxVM};
use cirru_parser::{parse, Cirru};

fn parse_program(source: &str) -> Result<Vec<CalxFunc>, String> {
  parse(source)
    .map_err(|error| error.to_string())?
    .into_iter()
    .map(|node| match node {
      Cirru::List(nodes) => parse_function(&nodes),
      Cirru::Leaf(_) => Err("expected a top-level function expression".to_string()),
    })
    .collect()
}

fn run_with_imports(source: &str, imports: CalxImportsDict) -> Result<Calx, String> {
  let mut vm = CalxVM::new(parse_program(source)?, vec![], imports);
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  vm.run(vec![]).map_err(|error| error.to_string())
}

fn run(source: &str) -> Result<Calx, String> {
  run_with_imports(source, HashMap::new())
}

fn add_import(args: &Vec<Calx>) -> Result<Calx, CalxError> {
  match args.as_slice() {
    [Calx::I64(left), Calx::I64(right)] => Ok(Calx::I64(left + right)),
    _ => Err(CalxError::new_raw(format!("add2 expected two integers, got {args:?}"))),
  }
}

#[test]
fn local_global_and_stack_opcodes_execute() -> Result<(), String> {
  let result = run(
    r#"fn main (-> i64)
  local.new $value
  const 4
  local.set $value
  local.get $value
  dup
  i.add
  global.new
  global.set 0
  global.get 0
  local.tee $value
  drop
  local.get $value
  return"#,
  )?;

  assert_eq!(result, Calx::I64(8));
  Ok(())
}

#[test]
fn integer_numeric_and_comparison_opcodes_execute() -> Result<(), String> {
  let cases = [
    ("const 6\n  const 7\n  i.mul", Calx::I64(42)),
    ("const 21\n  const 7\n  i.div", Calx::I64(3)),
    ("const 22\n  const 7\n  i.rem", Calx::I64(1)),
    ("const 7\n  i.neg", Calx::I64(-7)),
    ("const 1\n  const 3\n  i.shl", Calx::I64(8)),
    ("const 8\n  const 2\n  i.shr", Calx::I64(2)),
    ("const 7\n  const 7\n  i.eq", Calx::Bool(true)),
    ("const 7\n  const 8\n  i.ne", Calx::Bool(true)),
    ("const 7\n  const 8\n  i.lt", Calx::Bool(true)),
    ("const 7\n  const 7\n  i.le", Calx::Bool(true)),
    ("const 8\n  const 7\n  i.gt", Calx::Bool(true)),
    ("const 7\n  const 7\n  i.ge", Calx::Bool(true)),
  ];

  for (body, expected) in cases {
    let return_type = if matches!(expected, Calx::Bool(_)) { "bool" } else { "i64" };
    let source = format!("fn main (-> {return_type})\n  {body}\n  return");
    assert_eq!(run(&source)?, expected, "source:\n{source}");
  }
  Ok(())
}

#[test]
fn overloaded_and_float_opcodes_execute() -> Result<(), String> {
  let cases = [
    ("i64", "const 2\n  const 3\n  add", Calx::I64(5)),
    ("f64", "const 1.5\n  const 2.5\n  add", Calx::F64(4.0)),
    ("i64", "const 2\n  const 3\n  mul", Calx::I64(6)),
    ("f64", "const 1.5\n  const 2.\n  mul", Calx::F64(3.0)),
    ("f64", "const 5.\n  const 2.\n  div", Calx::F64(2.5)),
    ("f64", "const 2.5\n  neg", Calx::F64(-2.5)),
  ];

  for (return_type, body, expected) in cases {
    let source = format!("fn main (-> {return_type})\n  {body}\n  return");
    assert_eq!(run(&source)?, expected, "source:\n{source}");
  }
  Ok(())
}

#[test]
fn structured_control_call_and_import_opcodes_execute() -> Result<(), String> {
  let loop_result = run(
    r#"fn main (-> i64)
  const 0
  block (i64 -> i64)
    loop (i64 -> i64)
      const 1
      i.add
      dup
      const 3
      i.ge
      br-if 1
      br 0
  return"#,
  )?;
  assert_eq!(loop_result, Calx::I64(3));

  let if_result = run(
    r#"fn main (-> i64)
  const false
  if (-> i64)
    do
      const 1
    do
      const 2
  return"#,
  )?;
  assert_eq!(if_result, Calx::I64(2));

  let call_result = run(
    r#"fn identity (i64 -> i64)
  local.get 0
  return

fn main (-> i64)
  const 7
  call identity
  return"#,
  )?;
  assert_eq!(call_result, Calx::I64(7));

  let tail_call_result = run(
    r#"fn identity (i64 -> i64)
  local.get 0
  return

fn main (-> i64)
  const 8
  return-call identity"#,
  )?;
  assert_eq!(tail_call_result, Calx::I64(8));

  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(Rc::from("add2"), (add_import, 2));
  let import_result = run_with_imports(
    r#"fn main (-> i64)
  const 20
  const 22
  call-import add2
  return"#,
    imports,
  )?;
  assert_eq!(import_result, Calx::I64(42));
  Ok(())
}

#[test]
fn diagnostic_and_noop_opcodes_execute() -> Result<(), String> {
  let result = run(
    r#"fn main ()
  const true
  assert "|expected truthy"
  nop
  const "|opcode matrix echo"
  echo
  inspect
  return"#,
  )?;

  assert_eq!(result, Calx::Nil);
  Ok(())
}
