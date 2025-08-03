use calx_vm::{Calx, CalxFunc, CalxSyntax, CalxType, CalxVM};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{collections::HashMap, rc::Rc};

// Create a simple arithmetic function
fn create_arithmetic_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("arithmetic"),
    params_types: Rc::new(vec![CalxType::I64, CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      CalxSyntax::LocalGet(0),
      CalxSyntax::LocalGet(1),
      CalxSyntax::IntAdd,
      CalxSyntax::LocalGet(0),
      CalxSyntax::IntMul,
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["a".to_string(), "b".to_string()]),
  }
}

// Create a loop calculation function (sum from 1 to n)
fn create_sum_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("sum"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Initialize accumulator
      CalxSyntax::LocalNew,
      CalxSyntax::Const(Calx::I64(0)),
      CalxSyntax::LocalSet(1),
      // Initialize counter
      CalxSyntax::LocalNew,
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::LocalSet(2),
      // Loop start
      CalxSyntax::Block {
        looped: true,
        params_types: Rc::new(vec![]),
        ret_types: Rc::new(vec![]),
        from: 8,
        to: 20,
      },
      // Loop body: accumulator += counter
      CalxSyntax::LocalGet(1), // accumulator
      CalxSyntax::LocalGet(2), // counter
      CalxSyntax::IntAdd,
      CalxSyntax::LocalSet(1),
      // counter += 1
      CalxSyntax::LocalGet(2),
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::IntAdd,
      CalxSyntax::LocalTee(2),
      // Check if exceeds n
      CalxSyntax::LocalGet(0), // n
      CalxSyntax::IntGt,
      CalxSyntax::BrIf(0), // if counter > n, break loop
      CalxSyntax::Br(0),   // continue loop
      CalxSyntax::BlockEnd(true),
      // Return accumulator value
      CalxSyntax::LocalGet(1),
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["n".to_string(), "sum".to_string(), "i".to_string()]),
  }
}

// Create a recursive fibonacci function
fn create_fibonacci_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("fibonacci"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // if n <= 1 return n
      CalxSyntax::LocalGet(0),
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::IntLe,
      CalxSyntax::If {
        ret_types: Rc::new(vec![CalxType::I64]),
        else_at: 6,
        to: 8,
      },
      CalxSyntax::Do(vec![CalxSyntax::LocalGet(0)]),
      CalxSyntax::ThenEnd,
      CalxSyntax::Do(vec![
        // return fibonacci(n-1) + fibonacci(n-2)
        CalxSyntax::LocalGet(0),
        CalxSyntax::Const(Calx::I64(1)),
        CalxSyntax::IntAdd,
        CalxSyntax::Const(Calx::I64(-1)),
        CalxSyntax::IntMul,
        CalxSyntax::Call(Rc::from("fibonacci")),
        CalxSyntax::LocalGet(0),
        CalxSyntax::Const(Calx::I64(2)),
        CalxSyntax::IntAdd,
        CalxSyntax::Const(Calx::I64(-1)),
        CalxSyntax::IntMul,
        CalxSyntax::Call(Rc::from("fibonacci")),
        CalxSyntax::IntAdd,
      ]),
      CalxSyntax::ElseEnd,
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["n".to_string()]),
  }
}

// Create main function
fn create_main_func(call_target: &str) -> CalxFunc {
  CalxFunc {
    name: Rc::from("main"),
    params_types: Rc::new(vec![]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![CalxSyntax::Call(Rc::from(call_target)), CalxSyntax::Return]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![]),
  }
}

// Benchmark: simple arithmetic operations
fn bench_arithmetic(c: &mut Criterion) {
  let arithmetic_func = create_arithmetic_func();
  let main_func = create_main_func("arithmetic");
  let funcs = vec![main_func, arithmetic_func];
  let imports = HashMap::new();

  c.bench_function("arithmetic_operations", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![Calx::I64(10), Calx::I64(20)]).unwrap());
    })
  });
}

// Benchmark: loop calculation (sum from 1 to 1000)
fn bench_sum_loop(c: &mut Criterion) {
  let sum_func = create_sum_func();
  let main_func = create_main_func("sum");
  let funcs = vec![main_func, sum_func];
  let imports = HashMap::new();

  c.bench_function("sum_loop_1000", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![Calx::I64(1000)]).unwrap());
    })
  });
}

// Benchmark: recursive fibonacci (small values)
fn bench_fibonacci_recursive(c: &mut Criterion) {
  let fib_func = create_fibonacci_func();
  let main_func = create_main_func("fibonacci");
  let funcs = vec![main_func, fib_func];
  let imports = HashMap::new();

  c.bench_function("fibonacci_recursive_20", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![Calx::I64(20)]).unwrap());
    })
  });
}

// Benchmark: stack operations performance
fn bench_stack_operations(c: &mut Criterion) {
  let stack_func = CalxFunc {
    name: Rc::from("stack_ops"),
    params_types: Rc::new(vec![]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Lots of stack operations: push, dup, drop
      CalxSyntax::Const(Calx::I64(42)),
      CalxSyntax::Dup,
      CalxSyntax::Dup,
      CalxSyntax::Drop,
      CalxSyntax::Dup,
      CalxSyntax::Drop,
      CalxSyntax::Dup,
      CalxSyntax::Drop,
      CalxSyntax::Dup,
      CalxSyntax::Drop,
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![]),
  };

  let main_func = create_main_func("stack_ops");
  let funcs = vec![main_func, stack_func];
  let imports = HashMap::new();

  c.bench_function("stack_operations", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: VM initialization and preprocessing
fn bench_vm_initialization(c: &mut Criterion) {
  let arithmetic_func = create_arithmetic_func();
  let main_func = create_main_func("arithmetic");
  let funcs = vec![main_func, arithmetic_func];
  let imports = HashMap::new();

  c.bench_function("vm_initialization", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      black_box(());
      vm.setup_top_frame().unwrap();
      black_box(());
    })
  });
}

criterion_group!(
  benches,
  bench_arithmetic,
  bench_sum_loop,
  bench_fibonacci_recursive,
  bench_stack_operations,
  bench_vm_initialization
);
criterion_main!(benches);
