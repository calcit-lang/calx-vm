use std::fs;
use std::time::Instant;
use std::{collections::hash_map::HashMap, rc::Rc};

use argh::FromArgs;
use cirru_parser::{parse, Cirru};

use calx_vm::{log_calx_value, parse_function, Calx, CalxImportsDict, CalxVM};

// #[cfg(not(target_env = "msvc"))]
// use tikv_jemallocator::Jemalloc;

// #[cfg(not(target_env = "msvc"))]
// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

#[derive(FromArgs)]
/// Calx VM args
struct TopLevel {
  /// show code
  #[argh(switch, short = 's')]
  show_code: bool,
  /// verbose
  #[argh(switch, short = 'v')]
  verbose: bool,
  /// source
  #[argh(positional)]
  source: String,
}

fn main() -> Result<(), String> {
  let args: TopLevel = argh::from_env();

  let source = args.source;
  let show_code = args.show_code;

  let contents = fs::read_to_string(&source).map_err(|e| format!("failed to read `{source}`: {e}"))?;
  let xs = parse(&contents).map_err(|e| format!("failed to parse `{source}`: {e}"))?;
  let mut fns = vec![];

  for x in xs {
    if let Cirru::List(ys) = x {
      let f = parse_function(&ys)?;
      fns.push(f);
    } else {
      return Err("expected top-level function expressions".to_string());
    }
  }

  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(Rc::from("log"), (log_calx_value, 1));
  imports.insert(Rc::from("log2"), (log_calx_value, 2));
  imports.insert(Rc::from("log3"), (log_calx_value, 3));

  let mut vm = CalxVM::new(fns, vec![], imports);

  // if show_code {
  //   for func in vm.funcs.to_owned() {
  //     println!("loaded fn: {}", func);
  //   }
  // }

  let now = Instant::now();

  println!("[calx] start preprocessing");
  vm.preprocess(args.verbose)?;

  vm.setup_top_frame()?;

  if show_code {
    for func in &vm.funcs {
      println!("loaded fn: {func}");
    }
  }

  println!("[calx] start running");
  match vm.run(vec![Calx::I64(1)]) {
    Ok(ret) => {
      let elapsed = now.elapsed();

      println!("[calx] took {elapsed:.3?}: {ret:?}");
      Ok(())
    }
    Err(e) => {
      println!("VM state: {:?}", vm.stack);
      println!("{e}");
      Err(String::from("Failed to run."))
    }
  }
}
