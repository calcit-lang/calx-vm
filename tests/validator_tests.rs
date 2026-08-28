use std::collections::HashMap;

use calx_vm::{parse_function, validate_program, CalxFunc, CalxVM};
use cirru_parser::{parse, Cirru};

fn parse_program(source: &str) -> Result<Vec<CalxFunc>, String> {
  parse(source)
    .map_err(|e| e.to_string())?
    .into_iter()
    .map(|node| match node {
      Cirru::List(nodes) => parse_function(&nodes),
      Cirru::Leaf(_) => Err("expected a top-level function expression".to_string()),
    })
    .collect()
}

fn validate(source: &str) -> Result<(), String> {
  let funcs = parse_program(source)?;
  validate_program(&funcs, &[], &HashMap::new()).map_err(|e| e.to_string())
}

#[test]
fn rejects_wrong_numeric_operand_type() {
  let error = validate(
    r#"fn main ()
  const 1.0
  const 2
  i.add
  drop"#,
  )
  .expect_err("i.add must require two i64 operands");

  assert!(error.contains("expected I64, found F64"), "{error}");
  assert!(error.contains("main"), "{error}");
  assert!(error.contains("syntax[2]"), "{error}");
}

#[test]
fn rejects_wrong_call_argument_type() {
  let error = validate(
    r#"fn identity (i64 -> i64)
  local.get 0
  return

fn main ()
  const true
  call identity
  drop"#,
  )
  .expect_err("call must match the callee signature");

  assert!(error.contains("expected I64, found Bool"), "{error}");
}

#[test]
fn rejects_wrong_return_type() {
  let error = validate(
    r#"fn main (-> i64)
  const true
  return"#,
  )
  .expect_err("return must match the function signature");

  assert!(error.contains("expected I64, found Bool"), "{error}");
}

#[test]
fn rejects_wrong_block_result_type() {
  let error = validate(
    r#"fn main ()
  block (-> i64)
    const true
  drop"#,
  )
  .expect_err("block result must match its declared type");

  assert!(error.contains("expected I64, found Bool"), "{error}");
}

#[test]
fn rejects_wrong_branch_label_type() {
  let error = validate(
    r#"fn main ()
  block (-> i64)
    const true
    br 0
  drop"#,
  )
  .expect_err("branch values must match the target label type");

  assert!(error.contains("expected I64, found Bool"), "{error}");
}

#[test]
fn accepts_stack_polymorphism_in_unreachable_code() -> Result<(), String> {
  let unreachable_source = r#"fn main ()
  unreachable
  i.add
  drop"#;
  validate(unreachable_source)?;

  let branch_source = r#"fn main ()
  block (->)
    br 0
    i.add
    drop"#;
  validate(branch_source)?;

  for source in [unreachable_source, branch_source] {
    let funcs = parse_program(source)?;
    let mut vm = CalxVM::new(funcs, vec![], HashMap::new());
    vm.preprocess(false)?;
  }
  Ok(())
}

#[test]
fn preprocess_runs_validation_before_lowering() -> Result<(), String> {
  let funcs = parse_program(
    r#"fn main ()
  const 1.0
  const 2
  i.add
  drop"#,
  )?;
  let mut vm = CalxVM::new(funcs, vec![], HashMap::new());
  let error = vm.preprocess(false).expect_err("preprocess must reject invalid types");

  assert!(error.contains("expected I64, found F64"), "{error}");
  assert!(vm.funcs.iter().all(|func| func.instrs.is_empty()));
  Ok(())
}

#[test]
fn rejects_assignment_to_known_local_with_wrong_type() {
  let error = validate(
    r#"fn main (($value i64) ->)
  const true
  local.set $value"#,
  )
  .expect_err("typed function parameters must retain their declared type");

  assert!(error.contains("expected I64, found Bool"), "{error}");
}

#[test]
fn dynamic_local_defers_unknown_type_to_runtime() -> Result<(), String> {
  let source = r#"fn main ()
  local.new $value
  const true
  local.set $value
  local.get $value
  const 1
  i.add
  drop"#;

  let funcs = parse_program(source)?;
  validate_program(&funcs, &[], &HashMap::new()).map_err(|e| e.to_string())?;

  let mut vm = CalxVM::new(funcs, vec![], HashMap::new());
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  let error = vm.run(vec![]).expect_err("Dynamic must preserve the runtime type check");
  assert!(error.message.contains("expected 2 integers to add"), "{error}");
  Ok(())
}
