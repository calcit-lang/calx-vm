use std::collections::hash_map::HashMap;
use std::fs;
use std::time::Instant;

use cirru_parser::{parse, Cirru};
use clap::{Arg, Command};

use calx_vm::{log_calx_value, parse_function, Calx, CalxFunc, CalxImportsDict, CalxVM};

fn main() -> Result<(), String> {
  let matches = Command::new("Calx VM")
    .version("0.1.0")
    .author("Jon Chen <jiyinyiyong@gmail.com>")
    .about("A toy VM")
    .arg(
      Arg::new("SHOW_CODE")
        .short('S')
        .long("show-code")
        .value_name("show-code")
        .help("display processed instructions of functions")
        .takes_value(false),
    )
    .arg(
      Arg::new("DISABLE_PRE")
        .short('D')
        .long("disable-pre")
        .value_name("disable-pre")
        .help("disabled preprocess")
        .takes_value(false),
    )
    .arg(
      Arg::new("EMIT_BINARY")
        .long("emit-binary")
        .value_name("emit-binary")
        .help("emit binary, rather than running")
        .takes_value(true),
    )
    .arg(
      Arg::new("EVALUATE_BINARY")
        .long("eval-binary")
        .value_name("eval-binary")
        .help("evaluate program from binary")
        .takes_value(false),
    )
    .arg(Arg::new("SOURCE").help("A *.cirru file for loading code").required(true).index(1))
    .get_matches();

  let source = matches.value_of("SOURCE").unwrap();
  let show_code = matches.is_present("SHOW_CODE");
  let disable_pre = matches.is_present("DISABLE_PRE");
  let emit_binary = matches.is_present("EMIT_BINARY");
  let eval_binary = matches.is_present("EVALUATE_BINARY");

  let mut fns: Vec<CalxFunc> = vec![];

  if eval_binary {
    let code = fs::read(source).expect("read binar from source file");
    fns = bincode::decode_from_slice(&code, bincode::config::standard())
      .expect("decode functions from binary")
      .0;
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

  if emit_binary {
    let mut slice = [0u8; 10000];
    let length = match bincode::encode_into_slice(&fns, &mut slice, bincode::config::standard()) {
      Ok(l) => {
        println!("encoded binary length: {}", l);
        l
      }
      Err(e) => panic!("failed on default length of 10000: {}", e),
    };
    let slice = &slice[..length];
    let target_file = matches.value_of("EMIT_BINARY").unwrap();
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
