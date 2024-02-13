use std::fs;
use std::time::Instant;
use std::{collections::hash_map::HashMap, rc::Rc};

use cirru_parser::{parse, Cirru};
use clap::{arg, Parser};

use calx_vm::{log_calx_value, parse_function, Calx, CalxFunc, CalxImportsDict, CalxVM};

// #[cfg(not(target_env = "msvc"))]
// use tikv_jemallocator::Jemalloc;

// #[cfg(not(target_env = "msvc"))]
// #[global_allocator]
// static GLOBAL: Jemalloc = Jemalloc;

/// binary format for saving calx program
/// TODO this is not a valid file format that requires magic code
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxBinaryProgram {
  /// updates as instructions update
  pub edition: String,
  pub fns: Vec<CalxFunc>,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "Calx VM")]
#[command(author = "Jon Chen <jiyinyiyong@gmail.com>")]
#[command(version = "0.1.6")]
#[command(about = "A toy VM", long_about = None)]
struct Args {
  #[arg(short, long, value_name = "SHOW_CODE")]
  show_code: bool,
  #[arg(short, long, value_name = "EMIT_BINARY")]
  emit_binary: Option<String>,
  #[arg(short, long, value_name = "VERBOSE")]
  verbose: bool,
  #[arg(long, value_name = "EVAL_BINARY")]
  eval_binary: bool,
  #[arg(value_name = "SOURCE")]
  source: String,
}

fn main() -> Result<(), String> {
  let args = Args::parse();

  let source = args.source;
  let show_code = args.show_code;
  let emit_binary = args.emit_binary;
  let eval_binary = args.eval_binary;

  let mut fns: Vec<CalxFunc> = vec![];

  if eval_binary {
    todo!()
    // let code = fs::read(source).expect("read binary from source file");
    // let program: CalxBinaryProgram = bincode::decode_from_slice(&code, bincode::config::standard())
    //   .expect("decode functions from binary")
    //   .0;
    // if program.edition == CALX_INSTR_EDITION {
    //   println!("Calx Edition: {}", program.edition);
    //   fns = program.fns;
    // } else {
    //   return Err(format!(
    //     "Runner uses binary edition {}, binary encoded in {}",
    //     CALX_INSTR_EDITION, program.edition
    //   ));
    // }
  } else {
    let contents = fs::read_to_string(source).expect("Cirru file for instructions");
    let xs = parse(&contents).expect("Some Cirru content");

    for x in xs {
      if let Cirru::List(ys) = x {
        let f = parse_function(&ys)?;
        fns.push(f);
      } else {
        panic!("expected top level expressions");
      }
    }
  }

  if emit_binary.is_some() {
    todo!();
    // let program = CalxBinaryProgram {
    //   edition: CALX_INSTR_EDITION.to_string(),
    //   fns,
    // };
    // let buf = bincode::encode_to_vec(program, bincode::config::standard()).map_err(|e| e.to_string())?;
    // let target_file = &emit_binary.unwrap();
    // fs::write(target_file, buf).map_err(|e| e.to_string())?;
    // println!("wrote binary to {}", target_file);
    // return Ok(());
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
      println!("loaded fn: {}", func);
    }
  }

  println!("[calx] start running");
  match vm.run(vec![Calx::I64(1)]) {
    Ok(ret) => {
      let elapsed = now.elapsed();

      println!("[calx] took {:.3?}: {:?}", elapsed, ret);
      Ok(())
    }
    Err(e) => {
      println!("VM state: {:?}", vm.stack);
      println!("{}", e);
      Err(String::from("Failed to run."))
    }
  }
}
