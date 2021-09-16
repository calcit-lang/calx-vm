use std::collections::hash_map::HashMap;
use std::fs;

use cirru_parser::{parse, Cirru};

use calx_vm::{parse_function, Calx, CalxFunc, CalxImportsDict, CalxVM};

fn log2_calx_value(xs: Vec<Calx>) -> Result<Calx, String> {
  println!("log: {:?}", xs);
  Ok(Calx::Nil)
}

fn main() -> Result<(), String> {
  let contents = fs::read_to_string("examples/demo.cirru").expect("Cirru file for instructions");
  let code = parse(&contents).expect("Some Cirru content");

  if let Cirru::List(xs) = code {
    let mut fns: Vec<CalxFunc> = vec![];
    for x in xs {
      if let Cirru::List(ys) = x {
        let f = parse_function(&ys)?;
        fns.push(f);
      } else {
        panic!("TODO");
      }
    }

    let mut imports: CalxImportsDict = HashMap::new();
    imports.insert(String::from("log2"), (log2_calx_value, 2));

    let mut vm = CalxVM::new(fns, vec![], imports);

    // for func in vm.funcs.to_owned() {
    //   println!("loaded fn: {}", func);
    // }

    match vm.run() {
      Ok(()) => {
        println!("Result: {:?}", vm.stack);
        Ok(())
      }
      Err(e) => {
        println!("VM state: {:?}", vm.stack);
        println!("{}", e);
        Err(String::from("Failed to run"))
      }
    }
  } else {
    Err(String::from("TODO not cirru code"))
  }
}
