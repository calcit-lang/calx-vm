use std::fs;

use cirru_parser::{parse, Cirru};

use calx_vm::{parse_function, Calx, CalxFrame, CalxFunc, CalxVM};

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

    let mut vm = CalxVM::new(fns, vec![]);

    for func in vm.funcs.to_owned() {
      println!("loaded fn: {}", func);
    }

    match vm.run(0, vec![]) {
      Ok(_) => Ok(()),
      Err(e) => {
        println!("VM state: {:?}", vm.stack);
        Err(e)
      }
    }
  } else {
    Err(String::from("TODO not cirru code"))
  }
}
