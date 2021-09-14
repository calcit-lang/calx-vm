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

    let vm = CalxVM::new(fns, vec![]);

    println!("loaded vm: {:?}", vm);
    Ok(())
  } else {
    return Err(String::from("TODO not cirru code"));
  }
}
