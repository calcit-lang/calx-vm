use std::collections::HashMap;

use calx_vm::{parse_function, Calx, CalxFunc, CalxVM};
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

fn prepare_vm(source: &str) -> Result<CalxVM, String> {
  let mut vm = CalxVM::new(parse_program(source)?, vec![], HashMap::new());
  vm.preprocess(false)?;
  vm.setup_top_frame()?;
  Ok(vm)
}

fn run(source: &str) -> Result<Calx, String> {
  prepare_vm(source)?.run(vec![]).map_err(|e| e.to_string())
}

#[test]
fn local_tee_sets_local_and_preserves_stack_value() -> Result<(), String> {
  let result = run(
    r#"fn main (-> i64)
  local.new $value
  const 7
  local.tee $value
  local.get $value
  i.add
  return"#,
  )?;

  assert_eq!(result, Calx::I64(14));
  Ok(())
}

#[test]
fn global_set_accepts_valid_index_and_rejects_invalid_index() -> Result<(), String> {
  let result = run(
    r#"fn main (-> i64)
  global.new
  const 7
  global.set 0
  global.get 0
  return"#,
  )?;
  assert_eq!(result, Calx::I64(7));

  let error = run(
    r#"fn main ()
  const 7
  global.set 0"#,
  )
  .expect_err("global.set must reject an index that has not been allocated");
  assert!(error.contains("out of bound in global.set 0"), "{error}");
  Ok(())
}

#[test]
fn control_flow_uses_calx_truthiness_consistently() -> Result<(), String> {
  let result = run(
    r#"fn main (-> i64)
  const 2
  if (-> i64)
    do
      const 10
    do
      const 20
  return"#,
  )?;

  assert_eq!(result, Calx::I64(10));
  Ok(())
}

#[test]
fn branch_inside_if_resolves_enclosing_block() -> Result<(), String> {
  let result = run(
    r#"fn main ()
  block (->)
    const true
    if (->)
      do
        br 0
      do
        nop"#,
  )?;

  assert_eq!(result, Calx::Nil);
  Ok(())
}

#[test]
fn integer_operations_follow_wasm_style_wrapping_and_traps() -> Result<(), String> {
  let wrapped = run(
    r#"fn main (-> i64)
  const 9223372036854775807
  const 1
  i.add
  return"#,
  )?;
  assert_eq!(wrapped, Calx::I64(i64::MIN));

  let masked_shift = run(
    r#"fn main (-> i64)
  const 1
  const 64
  i.shl
  return"#,
  )?;
  assert_eq!(masked_shift, Calx::I64(1));

  let error = run(
    r#"fn main (-> i64)
  const 1
  const 0
  i.div
  return"#,
  )
  .expect_err("integer division by zero must trap");
  assert!(error.contains("trap: integer divide by zero"), "{error}");
  Ok(())
}

#[test]
fn unreachable_and_quit_are_reported_instead_of_panicking_or_exiting() -> Result<(), String> {
  let unreachable_error = run(
    r#"fn main ()
  unreachable"#,
  )
  .expect_err("unreachable must trap");
  assert!(unreachable_error.contains("trap: unreachable"), "{unreachable_error}");

  let quit_error = run(
    r#"fn main ()
  quit 7"#,
  )
  .expect_err("guest quit must not terminate the host process");
  assert!(quit_error.contains("status 7"), "{quit_error}");
  Ok(())
}

#[test]
fn explicit_void_return_produces_nil() -> Result<(), String> {
  let result = run(
    r#"fn main ()
  return"#,
  )?;
  assert_eq!(result, Calx::Nil);
  Ok(())
}

#[test]
fn reserved_instructions_are_rejected_by_parser() {
  for instruction in ["new-list", "list.get", "list.set", "new-link", "and", "or", "not"] {
    let source = format!("fn main ()\n  {instruction}");
    let error = parse_program(&source).expect_err("reserved instruction must not enter executable IR");
    assert!(error.contains("reserved but not implemented"), "{instruction}: {error}");
  }
}

#[test]
fn missing_main_is_a_setup_error_instead_of_a_constructor_panic() {
  let mut vm = CalxVM::new(vec![], vec![], HashMap::new());
  let error = vm.setup_top_frame().expect_err("missing main must be rejected");
  assert_eq!(error, "main function is required");
}
