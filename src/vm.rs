use crate::primes::{BlockData, Calx, CalxFrame, CalxFunc, CalxInstr};
use std::ops::Rem;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxVM {
  pub stack: Vec<Calx>,
  pub globals: Vec<Calx>,
  pub funcs: Vec<CalxFunc>,
  pub frames: Vec<CalxFrame>,
  pub top_frame: CalxFrame,
}

impl CalxVM {
  pub fn new(fns: Vec<CalxFunc>, globals: Vec<Calx>) -> Self {
    let main_func = find_func(&fns, "main").expect("main function is required");
    let main_frame = CalxFrame {
      initial_stack_size: 0,
      blocks_track: vec![],
      instrs: main_func.instrs,
      pointer: 0,
      locals: vec![],
    };
    CalxVM {
      stack: vec![],
      globals,
      funcs: fns,
      frames: vec![main_frame.to_owned()],
      top_frame: main_frame,
    }
  }

  pub fn get_instr(&mut self) -> Option<CalxInstr> {
    self
      .top_frame
      .instrs
      .get(self.top_frame.pointer)
      .map(|x| x.to_owned())
  }

  pub fn run(&mut self, idx: usize, params: Vec<Calx>) -> Result<Calx, String> {
    let mut result: Calx = Calx::Nil;

    loop {
      // println!("Stack {:?}", self.stack);
      let instr = self.get_instr();
      if instr == None {
        println!("no more instruction to run");
        break;
      }
      match instr.unwrap() {
        CalxInstr::Local => self.top_frame.locals.push(Calx::Nil),
        CalxInstr::LocalSet(idx) => {
          let v = self.stack_pop()?;
          if self.top_frame.locals.len() == idx {
            self.top_frame.locals.push(v)
          } else {
            self.top_frame.locals[idx] = v
          }
        }
        CalxInstr::LocalGet(idx) => {
          if idx < self.top_frame.locals.len() {
            self.stack.push(self.top_frame.locals[idx].to_owned())
          } else {
            return Err(format!("invalid index for local.get {}", idx));
          }
        }
        CalxInstr::GlobalSet(idx) => {
          let v = self.stack_pop()?;
          if self.globals.to_owned().len() == idx {
            self.globals.push(v)
          } else {
            self.globals[idx] = v
          }
        }
        CalxInstr::GlobalGet(idx) => {
          if idx < self.globals.len() {
            self.stack.push(self.globals[idx].to_owned())
          } else {
            return Err(format!("invalid index for local.get {}", idx));
          }
        }
        CalxInstr::Load(v) => {
          self.stack.push(v.to_owned());
        }
        CalxInstr::Dup => {
          let v = self.stack_peek()?;
          self.stack.push(v);
        }
        CalxInstr::Drop => {
          let _ = self.stack_pop()?;
        }
        CalxInstr::IntAdd => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack.push(Calx::I64(n1 + n2)),
            (_, _) => return Err(format!("expected 2 integers to add, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntMul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack.push(Calx::I64(n1 * n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to multiply, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::IntDiv => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack.push(Calx::I64(n1 / n2)),
            (_, _) => return Err(format!("expected 2 integers to divide, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntRem => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack.push(Calx::I64((*n1).rem(n2))),
            (_, _) => return Err(format!("expected 2 integers to add, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntNeg => {
          let v = self.stack_pop()?;
          if let Calx::I64(n) = v {
            self.stack.push(Calx::I64(-n))
          } else {
            return Err(format!("expected int, got {}", v));
          }
        }
        CalxInstr::IntShr => {
          // TODO
        }
        CalxInstr::IntShl => {
          // TODO
        }
        CalxInstr::Add => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack.push(Calx::F64(n1 + n2)),
            (_, _) => return Err(format!("expected 2 numbers to +, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Mul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack.push(Calx::F64(n1 * n2)),
            (_, _) => return Err(format!("expected 2 numbers to multiply, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Div => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack.push(Calx::F64(n1 / n2)),
            (_, _) => return Err(format!("expected 2 numbers to divide, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Neg => {
          let v = self.stack_pop()?;
          if let Calx::F64(n) = v {
            self.stack.push(Calx::F64(-n))
          } else {
            return Err(format!("expected float, got {}", v));
          }
        }
        CalxInstr::NewList => {
          // TODO
        }
        CalxInstr::ListGet => {
          // TODO
        }
        CalxInstr::ListSet => {
          // TODO
        }
        CalxInstr::NewLink => {
          // TODO
        }
        CalxInstr::And => {
          // TODO
        }
        CalxInstr::Or => {
          // TODO
        }
        CalxInstr::Br(usize) => {
          // TODO
        }
        CalxInstr::BrIf(usize) => {
          // TODO
        }
        CalxInstr::Block { looped, from, to } => {
          self
            .top_frame
            .blocks_track
            .push(BlockData { looped, from, to })
        }
        CalxInstr::BlockEnd => {
          self.top_frame.blocks_track.pop();
        }
        CalxInstr::Echo => {
          let v = self.stack_pop()?;
          println!("{}", v);
          // TODO
        }
        CalxInstr::Call(String, usize) => {
          // TODO
        }
        CalxInstr::Unreachable => {
          unreachable!("Unexpected from op")
        }
        CalxInstr::Nop => {
          // Noop
        }
        CalxInstr::Quit(code) => std::process::exit(code as i32),
        CalxInstr::Return => {
          // TODO
        }
      }

      self.top_frame.pointer += 1;
    }

    Ok(result)
  }

  fn stack_pop(&mut self) -> Result<Calx, String> {
    if self.stack.to_owned().is_empty() {
      Err(String::from("cannot pop from empty stack"))
    } else if self.stack.to_owned().len() <= self.top_frame.initial_stack_size {
      Err(String::from("cannot pop from parent stack"))
    } else {
      let v = self.stack.pop().unwrap();
      Ok(v)
    }
  }

  fn stack_peek(&mut self) -> Result<Calx, String> {
    if self.stack.is_empty() {
      Err(String::from("cannot peek empty stack"))
    } else if self.stack.len() <= self.top_frame.initial_stack_size {
      Err(String::from("cannot peek parent stack"))
    } else {
      Ok(self.stack.last().unwrap().to_owned())
    }
  }
}

pub fn find_func(funcs: &[CalxFunc], name: &str) -> Option<CalxFunc> {
  for x in funcs {
    if x.name == name {
      return Some(x.to_owned());
    }
  }
  None
}
