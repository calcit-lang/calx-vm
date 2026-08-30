use std::collections::HashMap;
use std::rc::Rc;

use calx_vm::{parse_function, Calx, CalxError, CalxFunc, CalxInstr, CalxSyntax, CalxVM};
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
  assert!(error.contains("global index 0 is not allocated"), "{error}");
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
fn branch_and_return_discard_intermediate_values() -> Result<(), String> {
  let branch_result = run(
    r#"fn main (-> i64)
  block (-> i64)
    const 99
    const 7
    br 0
  return"#,
  )?;
  assert_eq!(branch_result, Calx::I64(7));

  let conditional_branch_result = run(
    r#"fn main (-> i64)
  local.new $result
  const 7
  local.set $result
  block (-> i64)
    const 99
    local.get $result
    const true
    br-if 0
    drop
    drop
    local.get $result
  return"#,
  )?;
  assert_eq!(conditional_branch_result, Calx::I64(7));

  let return_result = run(
    r#"fn main (-> i64)
  const 99
  const 7
  return"#,
  )?;
  assert_eq!(return_result, Calx::I64(7));

  let tail_call_result = run(
    r#"fn identity (i64 -> i64)
  local.get 0
  return

fn main (-> i64)
  const 99
  const 7
  return-call identity"#,
  )?;
  assert_eq!(tail_call_result, Calx::I64(7));
  Ok(())
}

#[test]
fn function_fallthrough_returns_declared_result() -> Result<(), String> {
  let result = run(
    r#"fn main (-> i64)
  const 7"#,
  )?;
  assert_eq!(result, Calx::I64(7));
  Ok(())
}

#[test]
fn reserved_instructions_are_rejected_by_parser() {
  for instruction in ["new-list", "list.get", "list.set", "new-link", "and", "or", "not"] {
    let source = format!("fn main ()\n  {instruction}");
    let error = parse_program(&source).expect_err("reserved instruction must not enter executable IR");
    assert!(error.contains("reserved but not implemented"), "{instruction}: {error}");
  }

  for syntax in [
    CalxSyntax::NewList,
    CalxSyntax::ListGet,
    CalxSyntax::ListSet,
    CalxSyntax::NewLink,
    CalxSyntax::And,
    CalxSyntax::Or,
    CalxSyntax::Not,
  ] {
    let main = CalxFunc {
      name: Rc::from("main"),
      params_types: Rc::new(vec![]),
      ret_types: Rc::new(vec![]),
      syntax: Rc::new(vec![syntax]),
      instrs: Rc::new(vec![]),
      local_names: Rc::new(vec![]),
    };
    let mut vm = CalxVM::new(vec![main], vec![], HashMap::new());
    let error = vm
      .preprocess(false)
      .expect_err("reserved public syntax must be rejected by validation");
    assert!(error.contains("reserved but not implemented"), "{error}");
  }

  for instruction in [
    CalxInstr::NewList,
    CalxInstr::ListGet,
    CalxInstr::ListSet,
    CalxInstr::NewLink,
    CalxInstr::And,
    CalxInstr::Or,
    CalxInstr::Not,
  ] {
    let main = CalxFunc {
      name: Rc::from("main"),
      params_types: Rc::new(vec![]),
      ret_types: Rc::new(vec![]),
      syntax: Rc::new(vec![]),
      instrs: Rc::new(vec![instruction]),
      local_names: Rc::new(vec![]),
    };
    let mut vm = CalxVM::new(vec![main], vec![], HashMap::new());
    vm.setup_top_frame().expect("manually lowered main should be available");
    let error = vm
      .run(vec![])
      .expect_err("reserved public instruction must return an execution error");
    assert!(error.message.contains("unsupported instruction reached execution"), "{error}");
  }
}

#[test]
fn missing_main_is_a_setup_error_instead_of_a_constructor_panic() {
  let mut vm = CalxVM::new(vec![], vec![], HashMap::new());
  let error = vm.setup_top_frame().expect_err("missing main must be rejected");
  assert_eq!(error, "main function is required");
}

#[test]
fn calx_error_keeps_optional_vm_state_out_of_the_result_payload() -> Result<(), String> {
  assert!(std::mem::size_of::<CalxError>() <= 4 * std::mem::size_of::<usize>());

  let host_error = CalxError::new_raw("host failure".to_string());
  assert!(host_error.snapshot.is_none());
  assert_eq!(host_error.to_string(), "host failure");

  let vm_error = prepare_vm(
    r#"fn main ()
  unreachable"#,
  )?
  .run(vec![])
  .expect_err("runtime traps must retain a VM snapshot");
  assert_eq!(vm_error.top_frame().map(|frame| frame.name.as_ref()), Some("main"));
  assert_eq!(vm_error.stack(), Some([].as_slice()));
  assert_eq!(vm_error.globals(), Some([].as_slice()));
  Ok(())
}

#[test]
fn malformed_public_instructions_return_errors_instead_of_panicking() {
  for (instructions, expected) in [
    (vec![CalxInstr::Call(1)], "invalid function index for call: 1"),
    (vec![CalxInstr::ReturnCall(1)], "invalid function index for return-call: 1"),
    (
      vec![CalxInstr::JmpOffset(-1)],
      "instruction pointer moved before function start by offset -1",
    ),
    (
      vec![CalxInstr::Const(Calx::Bool(true)), CalxInstr::JmpOffsetIf(-2)],
      "instruction pointer moved before function start by offset -2",
    ),
  ] {
    let main = CalxFunc {
      name: Rc::from("main"),
      params_types: Rc::new(vec![]),
      ret_types: Rc::new(vec![]),
      syntax: Rc::new(vec![]),
      instrs: Rc::new(instructions),
      local_names: Rc::new(vec![]),
    };
    let mut vm = CalxVM::new(vec![main], vec![], HashMap::new());
    vm.setup_top_frame().expect("manually lowered main should be available");
    let error = vm.run(vec![]).expect_err("malformed public instruction must return an error");
    assert!(error.message.contains(expected), "expected {expected:?}, got {:?}", error.message);
  }
}
