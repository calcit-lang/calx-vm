use std::rc::Rc;

use calx_vm::{parse_program, Calx, CalxHostBindings, CalxRunResult, CalxVM, VmEvent, VmEventKind, VmObserver};

fn strict_vm(source: &str) -> CalxVM {
  let program = parse_program("tail-call.cirru", source).unwrap().into_program().unwrap();
  CalxVM::from_program(program, CalxHostBindings::new()).unwrap()
}

const MUTUAL: &str = r#"fn main (i64 -> i64)
  const 100
  local.get 0
  const 0
  call wide
  i.add
  return

fn wide (i64 i64 -> i64)
  local $a i64
  local $b i64
  local $c i64
  local.get 0
  const 0
  i.eq
  if (->)
    do
      local.get 1
      return
    do
  const 999
  local.get 0
  const -1
  i.add
  local.get 1
  local.get 0
  i.add
  const 77
  return-call narrow

fn narrow (i64 i64 i64 -> i64)
  local $flag bool
  local.get 0
  local.get 1
  return-call wide"#;

#[test]
fn self_tail_calls_discard_operands_and_reset_between_runs() {
  let mut vm = strict_vm(
    r#"fn main (i64 i64 -> i64)
  local $scratch i64
  local.get 0
  const 0
  i.eq
  if (->)
    do
      local.get 1
      return
    do
  const 123
  local.set $scratch
  const 999
  local.get 0
  const -1
  i.add
  local.get 1
  local.get 0
  i.add
  return-call main"#,
  );
  for n in [1000, 0, 10] {
    assert_eq!(
      vm.run_typed(vec![Calx::I64(n), Calx::I64(0)]).unwrap(),
      CalxRunResult::Value(Calx::I64(n * (n + 1) / 2))
    );
  }
}

#[derive(Default)]
struct Events(Vec<VmEvent>);

impl VmObserver for Events {
  fn on_event(&mut self, event: VmEvent) {
    self.0.push(event);
  }
}

#[test]
fn mutual_tail_calls_preserve_caller_stack_and_trace_depth_across_layouts() {
  let mut vm = strict_vm(MUTUAL);
  let mut events = Events::default();
  let result = vm.run_traced(vec![Calx::I64(4)], 1000, &mut events).unwrap();
  assert_eq!(result, CalxRunResult::Value(Calx::I64(110)));
  let tails: Vec<_> = events
    .0
    .iter()
    .filter(|e| matches!(e.kind, VmEventKind::Call { tail: true, .. }))
    .collect();
  assert_eq!(tails.len(), 8);
  for event in tails {
    assert_eq!(event.frame_depth_before, 1);
    assert_eq!(event.frame_depth_after, 1);
    assert_eq!(event.stack_after, vec![Calx::I64(100)]);
    let span = event.source_span.as_ref().unwrap();
    assert_eq!(&MUTUAL[span.start.offset..span.end.offset], "return-call");
  }
  let mut repeated = Events::default();
  assert_eq!(vm.run_traced(vec![Calx::I64(4)], 1000, &mut repeated).unwrap(), result);
  assert_eq!(repeated.0, events.0);
}

#[test]
fn new_layout_clears_initialized_slots_and_reports_callee_trap_span() {
  let source = r#"fn main (-> i64)
  local $old i64
  const 42
  local.set $old
  return-call fresh

fn fresh (-> i64)
  local $new i64
  local.get $new
  return"#;
  let mut vm = strict_vm(source);
  for _ in 0..2 {
    let error = vm.run_typed(vec![]).unwrap_err();
    assert!(error.message.contains("read before set"));
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.function, Some("fresh"));
    let span = diagnostic.span.unwrap();
    assert_eq!(&source[span.start.offset..span.end.offset], "local.get");
  }
}

#[test]
fn zero_arity_tail_call_releases_old_buffer_locals_and_discarded_operands() {
  let mut vm = strict_vm(
    r#"fn main (f64-buffer -> i64)
  local.get 0
  return-call finish

fn finish (-> i64)
  const 7
  return"#,
  );
  let buffer: Rc<[f64]> = Rc::from([1.0, 2.0]);
  assert_eq!(
    vm.run_typed(vec![Calx::f64_buffer_share(buffer.clone())]).unwrap(),
    CalxRunResult::Value(Calx::I64(7))
  );
  assert_eq!(Rc::strong_count(&buffer), 1);
}

#[test]
fn tail_call_moves_forwarded_buffer_before_releasing_old_frame() {
  let mut vm = strict_vm(
    r#"fn main (f64-buffer -> f64-buffer)
  local.get 0
  return-call identity

fn identity (f64-buffer -> f64-buffer)
  local.get 0
  return"#,
  );
  let buffer: Rc<[f64]> = Rc::from([2.0, 4.0]);
  let result = vm.run_typed(vec![Calx::f64_buffer_share(buffer.clone())]).unwrap();
  let CalxRunResult::Value(Calx::F64Buffer(result)) = result else {
    panic!("expected buffer result");
  };
  assert!(Rc::ptr_eq(&result, &buffer));
  drop(vm);
  assert_eq!(Rc::strong_count(&buffer), 2);
}

#[test]
fn legacy_tail_call_does_not_keep_previous_dynamic_locals() {
  let parsed = parse_program(
    "legacy-tail.cirru",
    r#"fn main (-> i64)
  local.new $old
  const 42
  local.set $old
  return-call fresh

fn fresh (-> i64)
  local.new $new
  local.get $new
  return"#,
  )
  .unwrap();
  let mut vm = CalxVM::new(parsed.functions, vec![], Default::default());
  vm.preprocess(false).unwrap();
  vm.setup_top_frame().unwrap();
  assert!(vm.run(vec![]).unwrap_err().message.contains("read before set"));
}
