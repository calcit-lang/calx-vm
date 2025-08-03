use calx_vm::{Calx, CalxFunc, CalxImportsDict, CalxSyntax, CalxType, CalxVM};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::{collections::HashMap, rc::Rc};

// Simple addition function
fn create_simple_add_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("add_test"),
    params_types: Rc::new(vec![CalxType::I64, CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      CalxSyntax::LocalGet(0),
      CalxSyntax::LocalGet(1),
      CalxSyntax::IntAdd,
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["a".to_string(), "b".to_string()]),
  }
}

// Complex arithmetic function
fn create_complex_arithmetic_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("complex_math"),
    params_types: Rc::new(vec![CalxType::I64, CalxType::I64, CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // (a + b) * c
      CalxSyntax::LocalGet(0), // a
      CalxSyntax::LocalGet(1), // b
      CalxSyntax::IntAdd,      // a + b
      CalxSyntax::LocalGet(2), // c
      CalxSyntax::IntMul,      // (a + b) * c
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
  }
}

// Stack operations function
fn create_stack_ops_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("stack_ops"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      CalxSyntax::LocalGet(0),         // Get parameter -> stack: [x]
      CalxSyntax::Dup,                 // Duplicate -> stack: [x, x]
      CalxSyntax::Const(Calx::I64(2)), // -> stack: [x, x, 2]
      CalxSyntax::IntMul,              // Multiply by 2 -> stack: [x, x*2]
      CalxSyntax::Dup,                 // Duplicate result -> stack: [x, x*2, x*2]
      CalxSyntax::Const(Calx::I64(3)), // -> stack: [x, x*2, x*2, 3]
      CalxSyntax::IntAdd,              // Add 3 -> stack: [x, x*2, x*2+3]
      CalxSyntax::Drop,                // Drop one value -> stack: [x, x*2]
      CalxSyntax::Drop,                // Drop another value -> stack: [x]
      CalxSyntax::Const(Calx::I64(1)), // -> stack: [x, 1]
      CalxSyntax::IntAdd,              // Add 1 -> stack: [x+1]
      CalxSyntax::Return,              // Return x+1
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["x".to_string()]),
  }
}

// Local variables function
fn create_local_vars_func() -> CalxFunc {
  CalxFunc {
    name: Rc::from("local_vars"),
    params_types: Rc::new(vec![CalxType::I64]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Create and operate on local variables
      CalxSyntax::LocalNew,    // Create local variable temp (index 1)
      CalxSyntax::LocalGet(0), // Get parameter
      CalxSyntax::Const(Calx::I64(10)),
      CalxSyntax::IntAdd,
      CalxSyntax::LocalSet(1), // temp = param + 10
      CalxSyntax::LocalGet(1), // Get temp
      CalxSyntax::Const(Calx::I64(5)),
      CalxSyntax::IntMul,      // temp * 5
      CalxSyntax::LocalTee(1), // Set temp and keep on stack
      CalxSyntax::LocalGet(0), // Get original parameter
      CalxSyntax::IntAdd,      // temp + param
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec!["param".to_string(), "temp".to_string()]),
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

// Benchmark: simple addition
fn bench_simple_add(c: &mut Criterion) {
  let add_func = create_simple_add_func();
  let main_func = create_main_func(
    "add_test",
    vec![CalxSyntax::Const(Calx::I64(100)), CalxSyntax::Const(Calx::I64(200))],
  );
  let funcs = vec![main_func, add_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("simple_add", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: complex arithmetic
fn bench_complex_arithmetic(c: &mut Criterion) {
  let math_func = create_complex_arithmetic_func();
  let main_func = create_main_func(
    "complex_math",
    vec![
      CalxSyntax::Const(Calx::I64(10)),
      CalxSyntax::Const(Calx::I64(20)),
      CalxSyntax::Const(Calx::I64(3)),
    ],
  );
  let funcs = vec![main_func, math_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("complex_arithmetic", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: stack operations
fn bench_stack_operations(c: &mut Criterion) {
  let stack_func = create_stack_ops_func();
  let main_func = create_main_func("stack_ops", vec![CalxSyntax::Const(Calx::I64(42))]);
  let funcs = vec![main_func, stack_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("stack_operations", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: local variables
fn bench_local_variables(c: &mut Criterion) {
  let local_func = create_local_vars_func();
  let main_func = create_main_func("local_vars", vec![CalxSyntax::Const(Calx::I64(15))]);
  let funcs = vec![main_func, local_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("local_variables", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: VM initialization and preprocessing overhead
fn bench_vm_setup(c: &mut Criterion) {
  let add_func = create_simple_add_func();
  let main_func = create_main_func("add_test", vec![CalxSyntax::Const(Calx::I64(10)), CalxSyntax::Const(Calx::I64(20))]);
  let funcs = vec![main_func, add_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("vm_setup", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      black_box(());
      vm.setup_top_frame().unwrap();
      black_box(());
    })
  });
}

// Benchmark: pure instruction execution (excluding initialization)
fn bench_instruction_execution(c: &mut Criterion) {
  let add_func = create_simple_add_func();
  let main_func = create_main_func("add_test", vec![CalxSyntax::Const(Calx::I64(42)), CalxSyntax::Const(Calx::I64(58))]);
  let funcs = vec![main_func, add_func];
  let imports: CalxImportsDict = HashMap::new();

  // Pre-setup VM
  let mut vm = CalxVM::new(funcs, vec![], imports);
  vm.preprocess(false).unwrap();
  vm.setup_top_frame().unwrap();

  c.bench_function("instruction_execution", |b| {
    b.iter(|| {
      let mut vm_clone = vm.clone();
      black_box(vm_clone.run(vec![]).unwrap());
    })
  });
}

// Benchmark: multiple function calls
fn bench_multiple_calls(c: &mut Criterion) {
  let add_func = create_simple_add_func();
  let caller_func = CalxFunc {
    name: Rc::from("caller"),
    params_types: Rc::new(vec![]),
    ret_types: Rc::new(vec![CalxType::I64]),
    syntax: Rc::new(vec![
      // Call add_test multiple times
      CalxSyntax::Const(Calx::I64(1)),
      CalxSyntax::Const(Calx::I64(2)),
      CalxSyntax::Call(Rc::from("add_test")),
      CalxSyntax::Const(Calx::I64(3)),
      CalxSyntax::IntAdd,
      CalxSyntax::Const(Calx::I64(4)),
      CalxSyntax::Call(Rc::from("add_test")),
      CalxSyntax::Return,
    ]),
    instrs: Rc::new(vec![]),
    local_names: Rc::new(vec![]),
  };

  let main_func = create_main_func("caller", vec![]);
  let funcs = vec![main_func, add_func, caller_func];
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("multiple_calls", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

criterion_group!(
  benches,
  bench_simple_add,
  bench_complex_arithmetic,
  bench_stack_operations,
  bench_local_variables,
  bench_vm_setup,
  bench_instruction_execution,
  bench_multiple_calls
);
criterion_main!(benches);
