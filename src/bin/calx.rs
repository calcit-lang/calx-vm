use calx_vm;

use calx_vm::Calx;

fn main() {
  println!("{}", Calx::Bool(true));
  println!("{}", Calx::Str(String::from("a")));
  println!("{}", Calx::F64(10.0));
  println!("{}", Calx::List(vec![Calx::Bool(true), Calx::I64(1)]));
}
