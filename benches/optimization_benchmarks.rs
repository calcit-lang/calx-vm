use calx_vm::{Calx, CalxFunc, CalxImportsDict, CalxSyntax, CalxType, CalxVM};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{collections::HashMap, rc::Rc};

// Create an arithmetic-intensive function to test optimization effects
fn create_arithmetic_intensive_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("arithmetic_intensive"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Execute intensive arithmetic operations: ((x + 1) * 2 + 3) * 4 + 5
      CalxSyntax::LocalGet(0), // x
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::IntAdd, // x + 1
      CalxSyntax::Const(Calx::I64(2)),
      CalxSyntax::IntMul, // (x + 1) * 2
      CalxSyntax::Const(Calx::I64(3)),
      CalxSyntax::IntAdd, // (x + 1) * 2 + 3
      CalxSyntax::Const(Calx::I64(4)),
      CalxSyntax::IntMul, // ((x + 1) * 2 + 3) * 4
      CalxSyntax::Const(Calx::I64(5)),
      CalxSyntax::IntAdd, // ((x + 1) * 2 + 3) * 4 + 5
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["x".to_string()]),
  }
}

// Create a stack-intensive function
fn create_stack_intensive_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("stack_intensive"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Intensive stack operations, ensure only one value remains on stack
      CalxSyntax::LocalGet(0),         // Get parameter -> stack: [x]
      CalxSyntax::Dup,                 // Duplicate -> stack: [x, x]
      CalxSyntax::Dup,                 // Duplicate -> stack: [x, x, x]
      CalxSyntax::Dup,                 // Duplicate -> stack: [x, x, x, x]
      CalxSyntax::Drop,                // Drop -> stack: [x, x, x]
      CalxSyntax::Drop,                // Drop -> stack: [x, x]
      CalxSyntax::Drop,                // Drop -> stack: [x]
      CalxSyntax::Dup,                 // Duplicate -> stack: [x, x]
      CalxSyntax::Const(Calx::I64(1)), // -> stack: [x, x, 1]
      CalxSyntax::IntAdd,              // Add 1 -> stack: [x, x+1]
      CalxSyntax::Drop,                // Drop x -> stack: [x+1]
      CalxSyntax::Return,              // Return x+1
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["x".to_string()]),
  }
}

// Create a locals-intensive function
fn create_locals_intensive_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("locals_intensive"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Create multiple local variables and access them frequently
      CalxSyntax::LocalNew,    // temp1 (index 1)
      CalxSyntax::LocalNew,    // temp2 (index 2)
      CalxSyntax::LocalNew,    // temp3 (index 3)
      CalxSyntax::LocalGet(0), // Get parameter
      CalxSyntax::LocalSet(1), // temp1 = param
      CalxSyntax::LocalGet(1), // Get temp1
      CalxSyntax::Const(Calx::I64(10)),
      CalxSyntax::IntAdd,
      CalxSyntax::LocalSet(2), // temp2 = temp1 + 10
      CalxSyntax::LocalGet(2), // Get temp2
      CalxSyntax::Const(Calx::I64(5)),
      CalxSyntax::IntMul,
      CalxSyntax::LocalSet(3), // temp3 = temp2 * 5
      CalxSyntax::LocalGet(1), // temp1
      CalxSyntax::LocalGet(2), // temp2
      CalxSyntax::IntAdd,
      CalxSyntax::LocalGet(3), // temp3
      CalxSyntax::IntAdd,      // temp1 + temp2 + temp3
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![
      "param".to_string(),
      "temp1".to_string(),
      "temp2".to_string(),
      "temp3".to_string(),
    ]),
  }
}

// Create a const-intensive function
fn create_const_intensive_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("const_intensive"),
    params_types: Rc::new(vec![]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Intensive constant operations
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::Const(Calx::I64(2)),
      CalxSyntax::IntAdd,
      CalxSyntax::Const(Calx::I64(3)),
      CalxSyntax::IntMul,
      CalxSyntax::Const(Calx::I64(4)),
      CalxSyntax::IntAdd,
      CalxSyntax::Const(Calx::I64(5)),
      CalxSyntax::IntMul,
      CalxSyntax::Const(Calx::I64(6)),
      CalxSyntax::IntAdd,
      CalxSyntax::Const(Calx::I64(7)),
      CalxSyntax::IntMul,
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![]),
  }
}

// Create main function
fn create_main_func(target_func: &str, args: Vec<CalxSyntax>) -> CalxFunc {
  let mut syntax = args;
  syntax.push(CalxSyntax::Call(Rc::from(target_func)));
  syntax.push(CalxSyntax::Return);

  CalxFunc {
    name: Rc::from("main"),
    params_types: Rc::new(vec![]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(syntax),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![]),
  }
}

// Benchmark: arithmetic-intensive operations
fn bench_arithmetic_intensive(c: &mut Criterion) {
  let arith_func = create_arithmetic_intensive_func();
  let main_func = create_main_func("arithmetic_intensive", vec![CalxSyntax::Const(Calx::I64(42))]);
  let funcs = vec![main_func, arith_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("arithmetic_intensive", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: stack-intensive operations
fn bench_stack_intensive(c: &mut Criterion) {
  let stack_func = create_stack_intensive_func();
  let main_func = create_main_func("stack_intensive", vec![CalxSyntax::Const(Calx::I64(100))]);
  let funcs = vec![main_func, stack_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("stack_intensive", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: locals-intensive operations
fn bench_locals_intensive(c: &mut Criterion) {
  let locals_func = create_locals_intensive_func();
  let main_func = create_main_func("locals_intensive", vec![CalxSyntax::Const(Calx::I64(25))]);
  let funcs = vec![main_func, locals_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("locals_intensive", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: const-intensive operations
fn bench_const_intensive(c: &mut Criterion) {
  let const_func = create_const_intensive_func();
  let main_func = create_main_func("const_intensive", vec![]);
  let funcs = vec![main_func, const_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("const_intensive", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: mixed operations (comprehensive test)
fn bench_mixed_operations(c: &mut Criterion) {
  let mixed_func = CalxFunc {
    name: Rc::from("mixed_ops"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Mix various operation types
      CalxSyntax::LocalNew,    // temp (index 1)
      CalxSyntax::LocalGet(0), // Get parameter
      CalxSyntax::Dup,         // Duplicate
      CalxSyntax::Const(Calx::I64(10)),
      CalxSyntax::IntAdd,      // param + 10
      CalxSyntax::LocalSet(1), // temp = param + 10
      CalxSyntax::Const(Calx::I64(2)),
      CalxSyntax::IntMul,      // (param + 10) * 2
      CalxSyntax::LocalGet(1), // Get temp
      CalxSyntax::IntAdd,      // Add temp
      CalxSyntax::Dup,         // Duplicate result
      CalxSyntax::Drop,        // Drop one
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["param".to_string(), "temp".to_string()]),
  };

  let main_func = create_main_func("mixed_ops", vec![CalxSyntax::Const(Calx::I64(15))]);
  let funcs = vec![main_func, mixed_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("mixed_operations", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

criterion_group!(
  optimization_benches,
  bench_arithmetic_intensive,
  bench_stack_intensive,
  bench_locals_intensive,
  bench_const_intensive,
  bench_mixed_operations
);
criterion_main!(optimization_benches);
