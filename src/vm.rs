use crate::primes::{BlockData, Calx, CalxFrame, CalxFunc, CalxInstr};
use std::collections::hash_map::HashMap;
use std::fmt;
use std::ops::Rem;

pub type CalxImportsDict = HashMap<String, (fn(xs: Vec<Calx>) -> Result<Calx, String>, usize)>;

#[derive(Clone)]
pub struct CalxVM {
  pub stack: Vec<Calx>,
  pub globals: Vec<Calx>,
  pub funcs: Vec<CalxFunc>,
  pub frames: Vec<CalxFrame>,
  pub top_frame: CalxFrame,
  pub imports: CalxImportsDict,
}

impl std::fmt::Debug for CalxVM {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("CalxVM Instance")
  }
}

impl CalxVM {
  pub fn new(fns: Vec<CalxFunc>, globals: Vec<Calx>, imports: CalxImportsDict) -> Self {
    let main_func = find_func(&fns, "main").expect("main function is required");
    let main_frame = CalxFrame {
      initial_stack_size: 0,
      blocks_track: vec![],
      instrs: main_func.instrs,
      pointer: 0,
      locals: vec![],
      ret_types: main_func.ret_types,
    };
    CalxVM {
      stack: vec![],
      globals,
      funcs: fns,
      frames: vec![],
      top_frame: main_frame,
      imports,
    }
  }

  pub fn run(&mut self) -> Result<(), String> {
    loop {
      // println!("Stack {:?}", self.stack);
      // println!("-- op {} {:?}", self.stack.len(), instr);

      if self.top_frame.pointer >= self.top_frame.instrs.len() {
        // println!("status {:?} {}", self.stack, self.top_frame);
        self.check_func_return()?;
        if self.frames.is_empty() {
          return Ok(());
        } else {
          // let prev_frame = self.top_frame;
          self.top_frame = self.frames.pop().unwrap();
        }
        self.top_frame.pointer += 1;
        continue;
      }
      match self.top_frame.instrs[self.top_frame.pointer].to_owned() {
        CalxInstr::Local => self.top_frame.locals.push(Calx::Nil),
        CalxInstr::LocalSet(idx) => {
          let v = self.stack_pop()?;
          if self.top_frame.locals.len() == idx {
            self.top_frame.locals.push(v)
          } else {
            self.top_frame.locals[idx] = v
          }
        }
        CalxInstr::LocalTee(idx) => {
          let v = self.stack_pop()?;
          if self.top_frame.locals.len() >= idx {
            return Err(format!("out of bound in local.set {}", idx));
          } else {
            self.top_frame.locals[idx] = v.to_owned()
          }
          self.stack_push(v);
        }
        CalxInstr::LocalGet(idx) => {
          if idx < self.top_frame.locals.len() {
            self.stack_push(self.top_frame.locals[idx].to_owned())
          } else {
            return Err(format!("invalid index for local.get {}", idx));
          }
        }
        CalxInstr::LocalNew => self.stack_push(Calx::Nil),
        CalxInstr::GlobalSet(idx) => {
          let v = self.stack_pop()?;
          if self.globals.len() >= idx {
            return Err(format!("out of bound in global.set {}", idx));
          } else {
            self.globals[idx] = v
          }
        }
        CalxInstr::GlobalGet(idx) => {
          if idx < self.globals.len() {
            self.stack_push(self.globals[idx].to_owned())
          } else {
            return Err(format!("invalid index for local.get {}", idx));
          }
        }
        CalxInstr::GlobalNew => self.globals.push(Calx::Nil),
        CalxInstr::Const(v) => {
          self.stack_push(v.to_owned());
        }
        CalxInstr::Dup => {
          let v = self.stack_peek()?;
          self.stack_push(v);
        }
        CalxInstr::Drop => {
          let _ = self.stack_pop()?;
        }
        CalxInstr::IntAdd => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::I64(n1 + n2)),
            (_, _) => return Err(format!("expected 2 integers to add, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntMul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::I64(n1 * n2)),
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
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::I64(n1 / n2)),
            (_, _) => return Err(format!("expected 2 integers to divide, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntRem => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::I64((*n1).rem(n2))),
            (_, _) => return Err(format!("expected 2 integers to add, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::IntNeg => {
          let v = self.stack_pop()?;
          if let Calx::I64(n) = v {
            self.stack_push(Calx::I64(-n))
          } else {
            return Err(format!("expected int, got {}", v));
          }
        }
        CalxInstr::IntShr => {
          let bits = self.stack_pop()?;
          let v = self.stack_pop()?;
          match (&v, &bits) {
            (Calx::I64(n), Calx::I64(b)) => {
              self.stack_push(Calx::I64(n.checked_shr(*b as u32).unwrap()))
            }
            (_, _) => return Err(format!("invalid number for SHR, {:?} {:?}", v, bits)),
          }
        }
        CalxInstr::IntShl => {
          let bits = self.stack_pop()?;
          let v = self.stack_pop()?;
          match (&v, &bits) {
            (Calx::I64(n), Calx::I64(b)) => {
              self.stack_push(Calx::I64(n.checked_shl(*b as u32).unwrap()))
            }
            (_, _) => return Err(format!("invalid number for SHL, {:?} {:?}", v, bits)),
          }
        }
        CalxInstr::IntEq => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 == n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to eq compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }

        CalxInstr::IntNe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 != n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to ne compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::IntLt => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 < n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to le compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::IntLe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 <= n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to le compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::IntGt => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 > n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to gt compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::IntGe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack_push(Calx::Bool(n1 >= n2)),
            (_, _) => {
              return Err(format!(
                "expected 2 integers to ge compare, {:?} {:?}",
                v1, v2
              ))
            }
          }
        }
        CalxInstr::Add => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack_push(Calx::F64(n1 + n2)),
            (_, _) => return Err(format!("expected 2 numbers to +, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Mul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack_push(Calx::F64(n1 * n2)),
            (_, _) => return Err(format!("expected 2 numbers to multiply, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Div => {
          // reversed order
          let v2 = self.stack_pop()?;
          let v1 = self.stack_pop()?;
          match (&v1, &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack_push(Calx::F64(n1 / n2)),
            (_, _) => return Err(format!("expected 2 numbers to divide, {:?} {:?}", v1, v2)),
          }
        }
        CalxInstr::Neg => {
          let v = self.stack_pop()?;
          if let Calx::F64(n) = v {
            self.stack_push(Calx::F64(-n))
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
        CalxInstr::Br(size) => {
          if self.top_frame.blocks_track.len() <= size {
            return Err(format!(
              "stack size {} eq/smaller than br size {}",
              self.top_frame.blocks_track.len(),
              size
            ));
          }

          self.shrink_blocks_by(size);

          let last_idx = self.top_frame.blocks_track.len() - 1;
          if self.top_frame.blocks_track[last_idx].looped {
            self.top_frame.pointer = self.top_frame.blocks_track[last_idx].from;
          } else {
            self.top_frame.pointer = self.top_frame.blocks_track[last_idx].to;
          }

          continue; // point reset, goto next loop
        }
        CalxInstr::BrIf(size) => {
          let v = self.stack_pop()?;
          if v == Calx::Bool(true) || v == Calx::I64(1) {
            if self.top_frame.blocks_track.len() <= size {
              return Err(format!(
                "stack size {} eq/smaller than br size {}",
                self.top_frame.blocks_track.len(),
                size
              ));
            }

            self.shrink_blocks_by(size);

            let last_idx = self.top_frame.blocks_track.len() - 1;
            if self.top_frame.blocks_track[last_idx].looped {
              self.top_frame.pointer = self.top_frame.blocks_track[last_idx].from;
            } else {
              self.top_frame.pointer = self.top_frame.blocks_track[last_idx].to;
            }

            continue; // point reset, goto next loop
          }
        }
        CalxInstr::Block {
          looped,
          from,
          to,
          params_types,
          ret_types,
        } => {
          if self.stack.len() < params_types.len() {
            return Err(format!(
              "no enough data on stack {:?} for {:?}",
              self.stack, params_types
            ));
          }
          self.top_frame.blocks_track.push(BlockData {
            looped,
            params_types: params_types.to_owned(),
            ret_types,
            from,
            to,
            initial_stack_size: self.stack.len() - params_types.len(),
          });
          println!("TODO check params type: {:?}", params_types);
        }
        CalxInstr::BlockEnd => {
          let last_block = self.top_frame.blocks_track.pop().unwrap();
          if self.stack.len() != last_block.initial_stack_size + last_block.ret_types.len() {
            return Err(format!(
              "block-end {:?} expected initial size {} plus {:?}, got stack size {}, in\n {}",
              last_block,
              last_block.initial_stack_size,
              last_block.ret_types,
              self.stack.len(),
              self.top_frame
            ));
          }
        }
        CalxInstr::Echo => {
          let v = self.stack_pop()?;
          println!("{}", v);
        }
        CalxInstr::Call(f_name) => {
          match find_func(&self.funcs, &f_name) {
            Some(f) => {
              let mut locals: Vec<Calx> = vec![];
              for _ in 0..f.params_types.len() {
                let v = self.stack_pop()?;
                locals.insert(0, v);
              }
              self.frames.push(self.top_frame.to_owned());
              self.top_frame = CalxFrame {
                blocks_track: vec![],
                initial_stack_size: self.stack.len(),
                locals,
                pointer: 0,
                instrs: f.instrs,
                ret_types: f.ret_types,
              };

              // TODO check params type
              println!("TODO check args: {:?}", f.params_types);

              // start in new frame
              continue;
            }
            None => return Err(format!("cannot find function named: {}", f_name)),
          }
        }
        CalxInstr::CallImport(f_name) => match self.imports.to_owned().get(&f_name) {
          None => return Err(format!("missing imported function {}", f_name)),
          Some((f, size)) => {
            if self.stack.len() < *size {
              return Err(format!(
                "imported function {} expected {} arguemtns, found {} on stack",
                f_name,
                size,
                self.stack.len()
              ));
            }
            let mut args: Vec<Calx> = vec![];
            for _ in 0..*size {
              args.insert(0, self.stack_pop()?);
            }
            let v = f(args)?;
            self.stack_push(v);
          }
        },
        CalxInstr::Unreachable => {
          unreachable!("Unexpected from op")
        }
        CalxInstr::Nop => {
          // Noop
        }
        CalxInstr::Quit(code) => std::process::exit(code as i32),
        CalxInstr::Return => {
          self.check_func_return()?;
          if self.frames.is_empty() {
            return Ok(());
          } else {
            // let prev_frame = self.top_frame;
            self.top_frame = self.frames.pop().unwrap();
          }
        }
      }

      self.top_frame.pointer += 1;
    }
  }

  #[inline(always)]
  fn check_func_return(&mut self) -> Result<(), String> {
    if self.stack.len() != self.top_frame.initial_stack_size + self.top_frame.ret_types.len() {
      return Err(format!(
        "stack size {} does not fit initial size {} plus {:?}",
        self.stack.len(),
        self.top_frame.initial_stack_size,
        self.top_frame.ret_types
      ));
    }

    Ok(())
  }

  #[inline(always)]
  fn stack_pop(&mut self) -> Result<Calx, String> {
    if self.stack.is_empty() {
      Err(String::from("cannot pop from empty stack"))
    } else if self.stack.len() <= self.top_frame.initial_stack_size {
      Err(String::from("cannot pop from parent stack"))
    } else {
      let v = self.stack.pop().unwrap();
      Ok(v)
    }
  }

  #[inline(always)]
  fn stack_push(&mut self, x: Calx) {
    self.stack.push(x)
  }

  #[inline(always)]
  fn stack_peek(&mut self) -> Result<Calx, String> {
    if self.stack.is_empty() {
      Err(String::from("cannot peek empty stack"))
    } else if self.stack.len() <= self.top_frame.initial_stack_size {
      Err(String::from("cannot peek parent stack"))
    } else {
      Ok(self.stack.last().unwrap().to_owned())
    }
  }

  /// assumed that the size already checked
  #[inline(always)]
  fn shrink_blocks_by(&mut self, size: usize) {
    let mut i = size;
    while i > 0 {
      self.top_frame.blocks_track.pop();
      i -= 1;
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
