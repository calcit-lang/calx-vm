use std::collections::hash_map::HashMap;
use std::fs;
use std::time::Instant;

use cirru_parser::{parse, Cirru};
use clap::{Arg, Command};

use calx_vm::{parse_function, Calx, CalxError, CalxFunc, CalxImportsDict, CalxVM};

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
    .arg(Arg::new("SOURCE").help("A *.cirru file for loading code").required(true).index(1))
    .get_matches();

  let source = matches.value_of("SOURCE").unwrap();
  let show_code = matches.is_present("SHOW_CODE");
  let disable_pre = matches.is_present("DISABLE_PRE");

  let contents = fs::read_to_string(source).expect("Cirru file for instructions");
  let xs = parse(&contents).expect("Some Cirru content");

  let mut fns: Vec<CalxFunc> = vec![];
  for x in xs {
    if let Cirru::List(ys) = x {
      let f = parse_function(&ys)?;
      fns.push(f);
    } else {
      panic!("expected top level expressions");
    }
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

  match vm.run() {
    Ok(()) => {
      let elapsed = now.elapsed();

      println!("Took {:.3?}: {:?}", elapsed, vm.stack);
      Ok(())
    }
    Err(e) => {
      println!("VM state: {:?}", vm.stack);
      println!("{}", e);
      Err(String::from("Failed to run."))
    }
  }
}

fn log_calx_value(xs: Vec<Calx>) -> Result<Calx, CalxError> {
  println!("log: {:?}", xs);
  Ok(Calx::Nil)
}
