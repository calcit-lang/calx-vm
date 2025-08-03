use calx_vm::parse_function;
use calx_vm::{CalxImportsDict, CalxVM};
use cirru_parser::parse;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::fs;

// Load and parse functions from demo files
fn load_demo_functions(demo_name: &str) -> Result<Vec<calx_vm::CalxFunc>, String> {
  let demo_path = format!("demos/{demo_name}.cirru");
  let contents = fs::read_to_string(&demo_path).map_err(|e| format!("Failed to read {demo_path}: {e}"))?;

  let xs = parse(&contents).map_err(|e| format!("Failed to parse {demo_path}: {e:?}"))?;

  let mut funcs = Vec::new();
  for x in xs {
    if let cirru_parser::Cirru::List(ys) = x {
      let f = parse_function(&ys)?;
      funcs.push(f);
    }
  }

  Ok(funcs)
}

// Benchmark: simple hello world
fn bench_hello_world(c: &mut Criterion) {
  let funcs = load_demo_functions("hello").expect("Failed to load hello demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("hello_world", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: if conditional judgment
fn bench_if_conditions(c: &mut Criterion) {
  let funcs = load_demo_functions("if").expect("Failed to load if demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("if_conditions", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: nested expressions
fn bench_nested_expressions(c: &mut Criterion) {
  let funcs = load_demo_functions("nested").expect("Failed to load nested demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("nested_expressions", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: recursion (small values)
fn bench_recursion_small(c: &mut Criterion) {
  let funcs = load_demo_functions("recur").expect("Failed to load recur demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("recursion_small", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: fibonacci (small values, avoid long execution time)
fn bench_fibonacci_10(c: &mut Criterion) {
  let mut funcs = load_demo_functions("fibonacci").expect("Failed to load fibonacci demo");

  // Modify main function to use smaller values
  for func in &mut funcs {
    if &*func.name == "main" {
      // Change fibonacci parameter from 34 to 10 to avoid long test time
      let new_syntax = func
        .syntax
        .iter()
        .map(|syntax| match syntax {
          calx_vm::CalxSyntax::Const(calx_vm::Calx::I64(34)) => calx_vm::CalxSyntax::Const(calx_vm::Calx::I64(10)),
          other => other.clone(),
        })
        .collect();

      func.syntax = std::rc::Rc::new(new_syntax);
      break;
    }
  }

  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("fibonacci_10", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: assertions
fn bench_assertions(c: &mut Criterion) {
  let funcs = load_demo_functions("assert").expect("Failed to load assert demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("assertions", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      vm.setup_top_frame().unwrap();
      black_box(vm.run(vec![]).unwrap());
    })
  });
}

// Benchmark: VM preprocessing overhead (using complex fibonacci)
fn bench_preprocessing_overhead(c: &mut Criterion) {
  let funcs = load_demo_functions("fibonacci").expect("Failed to load fibonacci demo");
  let imports: CalxImportsDict = HashMap::new();

  c.bench_function("preprocessing_overhead", |b| {
    b.iter(|| {
      let mut vm = CalxVM::new(funcs.clone(), vec![], imports.clone());
      vm.preprocess(false).unwrap();
      black_box(());
      vm.setup_top_frame().unwrap();
      black_box(());
    })
  });
}

// Benchmark: parsing overhead
fn bench_parsing_overhead(c: &mut Criterion) {
  let demo_path = "demos/fibonacci.cirru";
  let contents = fs::read_to_string(demo_path).expect("Failed to read fibonacci demo");

  c.bench_function("parsing_overhead", |b| {
    b.iter(|| {
      let xs = parse(&contents).expect("Failed to parse");
      let mut funcs = Vec::new();
      for x in xs {
        if let cirru_parser::Cirru::List(ys) = x {
          let f = parse_function(&ys).expect("Failed to parse function");
          funcs.push(f);
        }
      }
      black_box(funcs);
    })
  });
}

criterion_group!(
  benches,
  bench_hello_world,
  bench_if_conditions,
  bench_nested_expressions,
  bench_recursion_small,
  bench_fibonacci_10,
  bench_assertions,
  bench_preprocessing_overhead,
  bench_parsing_overhead
);
criterion_main!(benches);
