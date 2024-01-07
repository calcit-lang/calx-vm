use std::collections::hash_map::HashMap;
use std::fs;
use std::time::Instant;

use cirru_parser::{parse, Cirru};
use clap::{arg, Parser};

use calx_vm::{log_calx_value, parse_function, Calx, CalxBinaryProgram, CalxFunc, CalxImportsDict, CalxVM, CALX_BINARY_EDITION};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "Calx VM")]
#[command(author = "Jon Chen <jiyinyiyong@gmail.com>")]
#[command(version = "0.1.4")]
#[command(about = "A toy VM", long_about = None)]
struct Args {
  #[arg(short, long, value_name = "SHOW_CODE")]
  show_code: bool,
  #[arg(short, long, value_name = "DISABLE_PRE")]
  disable_pre: bool,
  #[arg(short, long, value_name = "EMIT_BINARY")]
  emit_binary: Option<String>,
  #[arg(long, value_name = "EVAL_BINARY")]
  eval_binary: bool,
  #[arg(value_name = "SOURCE")]
  source: String,
}

fn main() -> Result<(), String> {
  let args = Args::parse();

  let source = args.source;
  let show_code = args.show_code;
  let disable_pre = args.disable_pre;
  let emit_binary = args.emit_binary;
  let eval_binary = args.eval_binary;

  let mut fns: Vec<CalxFunc> = vec![];

  if eval_binary {
    let code = fs::read(source).expect("read binar from source file");
    let program: CalxBinaryProgram = bincode::decode_from_slice(&code, bincode::config::standard())
      .expect("decode functions from binary")
      .0;
    if program.edition == CALX_BINARY_EDITION {
      println!("Calx Edition: {}", program.edition);
      fns = program.fns;
    } else {
      return Err(format!(
        "Runner uses binary edition {}, binary encoded in {}",
        CALX_BINARY_EDITION, program.edition
      ));
    }
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
    let mut slice = [0u8; 10000];
    let program = CalxBinaryProgram {
      edition: CALX_BINARY_EDITION.to_string(),
      fns,
    };
    let length = match bincode::encode_into_slice(&program, &mut slice, bincode::config::standard()) {
      Ok(l) => {
        println!("encoded binary length: {}", l);
        l
      }
      Err(e) => panic!("failed on default length of 10000: {}", e),
    };
    let slice = &slice[..length];
    let target_file = &emit_binary.unwrap();
    match fs::write(target_file, slice) {
      Ok(_) => println!("wrote binary to {}", target_file),
      Err(e) => panic!("failed to write binary to {}: {}", target_file, e),
    };
    return Ok(());
    // println!("Bytes written: {:?}", slice);
  }

  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(String::from("log"), (log_calx_value, 1));
  imports.insert(String::from("log2"), (log_calx_value, 2));
  imports.insert(String::from("log3"), (log_calx_value, 3));

  let mut vm = CalxVM::new(fns, vec![], imports);

  // if show_code {
  //   for func in vm.funcs.to_owned() {
  //     println!("loaded fn: {}", func);
  //   }
  // }

  let now = Instant::now();
  if !disable_pre {
    vm.preprocess()?;
  } else {
    println!("Preprocess disabled.")
  }

  if show_code {
    for func in &vm.funcs {
      println!("loaded fn: {}", func);
    }
  }

  match vm.run(vec![Calx::I64(1)]) {
    Ok(ret) => {
      let elapsed = now.elapsed();

      println!("Took {:.3?}: {:?}", elapsed, ret);
      Ok(())
    }
    Err(e) => {
      println!("VM state: {:?}", vm.stack);
      println!("{}", e);
      Err(String::from("Failed to run."))
    }
  }
}
