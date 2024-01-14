use crate::primes::{BlockData, Calx, CalxError, CalxFrame, CalxFunc, CalxInstr, CalxType};
use std::collections::hash_map::HashMap;
use std::ops::Rem;
use std::rc::Rc;
use std::{fmt, vec};

pub type CalxImportsDict = HashMap<String, (fn(xs: Vec<Calx>) -> Result<Calx, CalxError>, usize)>;

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
    let mut fns = fns.to_owned();
    let main_func = find_func(&mut fns, "main").expect("main function is required");
    let main_frame = CalxFrame {
      initial_stack_size: 0,
      blocks_track: vec![],
      instrs: main_func.instrs.to_owned(),
      pointer: 0,
      locals: vec![],
      ret_types: main_func.ret_types.to_owned(),
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

  pub fn run(&mut self, args: Vec<Calx>) -> Result<Calx, CalxError> {
    // assign function parameters
    self.top_frame.locals = args;
    self.stack.clear();
    loop {
      // println!("Stack {:?}", self.stack);
      // println!("-- op {} {:?}", self.stack.len(), instr);

      if self.top_frame.pointer >= self.top_frame.instrs.len() {
        // println!("status {:?} {}", self.stack, self.top_frame);
        self.check_func_return()?;
        if self.frames.is_empty() {
          return Ok(self.stack.pop().unwrap_or(Calx::Nil));
        } else {
          // let prev_frame = self.top_frame;
          self.top_frame = self.frames.pop().unwrap();
        }
        self.top_frame.pointer += 1;
        continue;
      }
      match self.top_frame.instrs[self.top_frame.pointer].to_owned() {
        CalxInstr::Jmp(line) => {
          self.top_frame.pointer = line;
          continue; // point reset, goto next loop
        }
        CalxInstr::JmpIf(line) => {
          let v = self.stack_pop()?;
          if v == Calx::Bool(true) || v == Calx::I64(1) {
            self.top_frame.pointer = line;
            continue; // point reset, goto next loop
          }
        }
        CalxInstr::Br(size) => {
          self.shrink_blocks_by(size)?;

          let last_idx = self.top_frame.blocks_track.len() - 1;
          if self.top_frame.blocks_track[last_idx].looped {
            self.top_frame.pointer = self.top_frame.blocks_track[last_idx].from;
          } else {
            self.top_frame.pointer = self.top_frame.blocks_track[last_idx].to;
          }

          continue; // point reset, goto next loop
        }
        CalxInstr::BrIf(size) => {
          let last_idx = self.stack.len() - 1;
          if self.stack[last_idx] == Calx::Bool(true) || self.stack[last_idx] == Calx::I64(1) {
            self.shrink_blocks_by(size)?;

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
            return Err(self.gen_err(format!("no enough data on stack {:?} for {:?}", self.stack, params_types)));
          }
          self.top_frame.blocks_track.push(BlockData {
            looped,
            params_types: params_types.to_owned(),
            ret_types,
            from,
            to,
            initial_stack_size: self.stack.len() - params_types.len(),
          });
          self.check_stack_for_block(&params_types)?;
        }
        CalxInstr::BlockEnd(looped) => {
          if looped {
            return Err(self.gen_err(String::from("loop end expected to be branched")));
          }
          let last_block = self.top_frame.blocks_track.pop().unwrap();
          if self.stack.len() != last_block.initial_stack_size + last_block.ret_types.len() {
            return Err(self.gen_err(format!(
              "block-end {:?} expected initial size {} plus {:?}, got stack size {}, in\n {}",
              last_block,
              last_block.initial_stack_size,
              last_block.ret_types,
              self.stack.len(),
              self.top_frame
            )));
          }
        }
        CalxInstr::LocalSet(idx) => {
          let v = self.stack_pop()?;
          if idx >= self.top_frame.locals.len() {
            return Err(self.gen_err(format!("out of bound in local.set {} for {:?}", idx, self.top_frame.locals)));
          } else {
            self.top_frame.locals[idx] = v
          }
        }
        CalxInstr::LocalTee(idx) => {
          let v = self.stack_pop()?;
          if idx >= self.top_frame.locals.len() {
            return Err(self.gen_err(format!("out of bound in local.tee {}", idx)));
          } else {
            self.top_frame.locals[idx] = v.to_owned()
          }
          self.stack_push(v);
        }
        CalxInstr::LocalGet(idx) => {
          if idx < self.top_frame.locals.len() {
            self.stack_push(self.top_frame.locals[idx].to_owned())
          } else {
            return Err(self.gen_err(format!("invalid index for local.get {}", idx)));
          }
        }
        CalxInstr::Return => {
          self.check_func_return()?;
          if self.frames.is_empty() {
            return match self.stack.pop() {
              Some(x) => Ok(x),
              None => Err(self.gen_err("return without value".to_owned())),
            };
          } else {
            // let prev_frame = self.top_frame;
            self.top_frame = self.frames.pop().unwrap();
          }
        }
        CalxInstr::LocalNew => self.top_frame.locals.push(Calx::Nil),
        CalxInstr::GlobalSet(idx) => {
          let v = self.stack_pop()?;
          if self.globals.len() >= idx {
            return Err(self.gen_err(format!("out of bound in global.set {}", idx)));
          } else {
            self.globals[idx] = v
          }
        }
        CalxInstr::GlobalGet(idx) => {
          if idx < self.globals.len() {
            self.stack_push(self.globals[idx].to_owned())
          } else {
            return Err(self.gen_err(format!("invalid index for global.get {}", idx)));
          }
        }
        CalxInstr::GlobalNew => self.globals.push(Calx::Nil),
        CalxInstr::Const(v) => self.stack_push(v.to_owned()),
        CalxInstr::Dup => {
          self.stack_push(self.stack[self.stack.len() - 1].to_owned());
        }
        CalxInstr::Drop => {
          let _ = self.stack_pop()?;
        }
        CalxInstr::IntAdd => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&(self.stack[last_idx]), &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 + n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to add, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntMul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 * n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to multiply, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntDiv => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 / n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to divide, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntRem => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64((*n1).rem(n2)),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to add, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntNeg => {
          let last_idx = self.stack.len() - 1;
          if let Calx::I64(n) = self.stack[last_idx] {
            self.stack[last_idx] = Calx::I64(-n)
          } else {
            return Err(self.gen_err(format!("expected int, got {}", self.stack[last_idx])));
          }
        }
        CalxInstr::IntShr => {
          let bits = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &bits) {
            (Calx::I64(n), Calx::I64(b)) => self.stack[last_idx] = Calx::I64(n.checked_shr(*b as u32).unwrap()),
            (_, _) => return Err(self.gen_err(format!("invalid number for SHR, {:?} {:?}", self.stack[last_idx], bits))),
          }
        }
        CalxInstr::IntShl => {
          let bits = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &bits) {
            (Calx::I64(n), Calx::I64(b)) => self.stack[last_idx] = Calx::I64(n.checked_shl(*b as u32).unwrap()),
            (_, _) => return Err(self.gen_err(format!("invalid number for SHL, {:?} {:?}", self.stack[last_idx], bits))),
          }
        }
        CalxInstr::IntEq => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 == n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to eq compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }

        CalxInstr::IntNe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 != n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to ne compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntLt => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 < n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to le compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntLe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;
          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 <= n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to le compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntGt => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 > n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to gt compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::IntGe => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 >= n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 integers to ge compare, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::Add => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 + n2),
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 + n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 numbers to +, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::Mul => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 * n2),
            (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 * n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 numbers to multiply, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::Div => {
          // reversed order
          let v2 = self.stack_pop()?;
          let last_idx = self.stack.len() - 1;

          match (&self.stack[last_idx], &v2) {
            (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 / n2),
            (_, _) => return Err(self.gen_err(format!("expected 2 numbers to divide, {:?} {:?}", self.stack[last_idx], v2))),
          }
        }
        CalxInstr::Neg => {
          let last_idx = self.stack.len() - 1;
          if let Calx::F64(n) = self.stack[last_idx] {
            self.stack[last_idx] = Calx::F64(-n)
          } else {
            return Err(self.gen_err(format!("expected float, got {}", self.stack[last_idx])));
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
        CalxInstr::Not => {
          // TODO
        }
        CalxInstr::Call(f_name) => {
          // println!("frame size: {}", self.frames.len());
          match find_func(&mut self.funcs, &f_name) {
            Some(f) => {
              let instrs = f.instrs.to_owned();
              let ret_types = f.ret_types.to_owned();
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
                instrs,
                ret_types,
              };

              // start in new frame
              continue;
            }
            None => return Err(self.gen_err(format!("cannot find function named: {}", f_name))),
          }
        }
        CalxInstr::ReturnCall(f_name) => {
          // println!("frame size: {}", self.frames.len());
          match find_func(&mut self.funcs, &f_name) {
            Some(f) => {
              // println!("examine stack: {:?}", self.stack);
              let instrs = f.instrs.to_owned();
              let ret_types = f.ret_types.to_owned();
              let mut locals: Vec<Calx> = vec![];
              for _ in 0..f.params_types.len() {
                let v = self.stack_pop()?;
                locals.insert(0, v);
              }
              let prev_frame = self.top_frame.to_owned();
              if prev_frame.initial_stack_size != self.stack.len() {
                return Err(self.gen_err(format!(
                  "expected constant initial stack size: {}, got: {}",
                  prev_frame.initial_stack_size,
                  self.stack.len()
                )));
              }
              self.top_frame = CalxFrame {
                blocks_track: vec![],
                initial_stack_size: self.stack.len(),
                locals,
                pointer: 0,
                instrs,
                ret_types,
              };

              // start in new frame
              continue;
            }
            None => return Err(self.gen_err(format!("cannot find function named: {}", f_name))),
          }
        }
        CalxInstr::CallImport(f_name) => match self.imports.to_owned().get(&*f_name) {
          None => return Err(self.gen_err(format!("missing imported function {}", f_name))),
          Some((f, size)) => {
            if self.stack.len() < *size {
              return Err(self.gen_err(format!(
                "imported function {} expected {} arguemtns, found {} on stack",
                f_name,
                size,
                self.stack.len()
              )));
            }
            let mut args: Vec<Calx> = vec![];
            for _ in 0..*size {
              let item = self.stack_pop()?;
              args.insert(0, item);
            }
            let v = f(args.to_owned())?;
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
        CalxInstr::Echo => {
          let v = self.stack_pop()?;
          println!("{}", v);
        }
        CalxInstr::Assert(message) => {
          let v = self.stack_pop()?;
          if v == Calx::Bool(true) || v == Calx::I64(1) {
            // Ok
          } else {
            return Err(self.gen_err(format!("Failed assertion: {}", message)));
          }
        }
      }

      self.top_frame.pointer += 1;
    }
  }

  pub fn preprocess(&mut self) -> Result<(), String> {
    for i in 0..self.funcs.len() {
      let mut stack_size = 0;
      let mut ops: Vec<CalxInstr> = vec![];
      let mut blocks_track: Vec<BlockData> = vec![];

      // println!("\nFUNC {} {}", self.funcs[i].name, stack_size);

      for j in 0..self.funcs[i].instrs.len() {
        // println!("{} * {:?}", stack_size, self.funcs[i].instrs[j].to_owned());
        match self.funcs[i].instrs[j].to_owned() {
          CalxInstr::Block {
            looped,
            params_types,
            ret_types,
            from,
            to,
          } => {
            if stack_size < params_types.len() {
              return Err(format!("insufficient params {} for block: {:?}", stack_size, params_types));
            }
            blocks_track.push(BlockData {
              looped,
              params_types,
              ret_types,
              from,
              to,
              initial_stack_size: stack_size,
            });
            ops.push(CalxInstr::Nop);
          }
          CalxInstr::Br(size) => {
            if size > blocks_track.len() {
              return Err(format!("br {} too large", size));
            }

            let target_block = blocks_track[blocks_track.len() - size - 1].to_owned();
            if target_block.looped {
              // not checking
              ops.push(CalxInstr::Jmp(target_block.from))
            } else {
              ops.push(CalxInstr::Jmp(target_block.to))
            }
          }
          CalxInstr::BrIf(size) => {
            if blocks_track.is_empty() {
              return Err(format!("cannot branch with no blocks, {}", size));
            }
            if size > blocks_track.len() {
              return Err(format!("br {} too large", size));
            }

            let target_block = blocks_track[blocks_track.len() - size - 1].to_owned();
            if target_block.looped {
              // not checking
              ops.push(CalxInstr::JmpIf(target_block.from))
            } else {
              ops.push(CalxInstr::JmpIf(target_block.to))
            }
            stack_size -= 1
          }
          CalxInstr::BlockEnd(looped) => {
            // println!("checking: {:?}", blocks_track);
            if blocks_track.is_empty() {
              return Err(format!("invalid block end, {:?}", blocks_track));
            }

            let prev_block = blocks_track.pop().unwrap();
            if looped {
              // nothing, branched during runtime
            } else if stack_size != prev_block.initial_stack_size + prev_block.ret_types.len() - prev_block.params_types.len() {
              return Err(format!(
                "stack size is {stack_size}, initial size is {}, return types is {:?} at block end",
                prev_block.initial_stack_size, prev_block.ret_types
              ));
            }

            ops.push(CalxInstr::Nop)
          }
          CalxInstr::Call(f_name) => match find_func(&mut self.funcs, &f_name) {
            Some(f) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::Call(f_name))
            }
            None => return Err(format!("cannot find function named: {}", f_name)),
          },
          CalxInstr::ReturnCall(f_name) => match find_func(&mut self.funcs, &f_name) {
            Some(f) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::ReturnCall(f_name))
            }
            None => return Err(format!("cannot find function named: {}", f_name)),
          },
          CalxInstr::CallImport(f_name) => match &self.imports.get(&*f_name) {
            Some((_f, size)) => {
              if stack_size < *size {
                return Err(format!("insufficient size to call import: {} {:?}", stack_size, size));
              }
              stack_size = stack_size - size + 1;
              ops.push(CalxInstr::CallImport(f_name))
            }
            None => return Err(format!("missing imported function {}", f_name)),
          },
          CalxInstr::Return => {
            if stack_size != self.funcs[i].ret_types.len() {
              return Err(format!(
                "invalid return size {} for {:?} in {}",
                stack_size, self.funcs[i].ret_types, self.funcs[i].name
              ));
            }
            ops.push(CalxInstr::Return);
          }
          a => {
            // checks
            let (params_size, ret_size) = instr_stack_arity(&a);
            if stack_size < params_size {
              return Err(format!("insufficient stack {} to call {:?} of {}", stack_size, a, params_size));
            }
            stack_size = stack_size - params_size + ret_size;
            // println!(
            //   "  sizes: {:?} {} {} -> {}",
            //   a, params_size, ret_size, stack_size
            // );
            ops.push(a.to_owned());
          }
        }
      }
      if stack_size != self.funcs[i].ret_types.len() {
        return Err(format!(
          "invalid return size {} of {:?} in {}",
          stack_size, self.funcs[i].ret_types, self.funcs[i].name
        ));
      }

      self.funcs[i].instrs = Rc::new(ops);
    }
    Ok(())
  }

  /// checks is given parameters on stack top
  fn check_stack_for_block(&self, params: &[CalxType]) -> Result<(), CalxError> {
    if self.stack.len() < params.len() {
      return Err(self.gen_err(format!("stack size does not match given params: {:?} {:?}", self.stack, params)));
    }
    for (idx, t) in params.iter().enumerate() {
      if self.stack[self.stack.len() - params.len() - 1 + idx].typed_as(t.to_owned()) {
        continue;
      }
      return Err(self.gen_err(format!("stack type does not match given params: {:?} {:?}", self.stack, params)));
    }
    Ok(())
  }

  #[inline(always)]
  fn check_func_return(&mut self) -> Result<(), CalxError> {
    if self.stack.len() != self.top_frame.initial_stack_size + self.top_frame.ret_types.len() {
      return Err(self.gen_err(format!(
        "stack size {} does not fit initial size {} plus {:?}",
        self.stack.len(),
        self.top_frame.initial_stack_size,
        self.top_frame.ret_types
      )));
    }

    Ok(())
  }

  #[inline(always)]
  fn stack_pop(&mut self) -> Result<Calx, CalxError> {
    if self.stack.is_empty() {
      Err(self.gen_err(String::from("cannot pop from empty stack")))
    } else if self.stack.len() <= self.top_frame.initial_stack_size {
      Err(self.gen_err(String::from("cannot pop from parent stack")))
    } else {
      let v = self.stack.pop().unwrap();
      Ok(v)
    }
  }

  #[inline(always)]
  fn stack_push(&mut self, x: Calx) {
    self.stack.push(x)
  }

  /// assumed that the size already checked
  #[inline(always)]
  fn shrink_blocks_by(&mut self, size: usize) -> Result<(), CalxError> {
    if self.top_frame.blocks_track.len() <= size {
      return Err(self.gen_err(format!(
        "stack size {} eq/smaller than br size {}",
        self.top_frame.blocks_track.len(),
        size
      )));
    }

    let mut i = size;
    while i > 0 {
      self.top_frame.blocks_track.pop();
      i -= 1;
    }

    Ok(())
  }

  fn gen_err(&self, s: String) -> CalxError {
    CalxError {
      message: s,
      blocks: self.top_frame.blocks_track.to_owned(),
      top_frame: self.top_frame.to_owned(),
      stack: self.stack.to_owned(),
      globals: self.globals.to_owned(),
    }
  }
}

pub fn find_func<'a>(funcs: &'a mut [CalxFunc], name: &str) -> Option<&'a mut CalxFunc> {
  funcs.iter_mut().find(|x| &*x.name == name)
}

/// notice that some of the instrs are special and need to handle manually
pub fn instr_stack_arity(op: &CalxInstr) -> (usize, usize) {
  match op {
    CalxInstr::LocalSet(_) => (1, 0),
    CalxInstr::LocalTee(_) => (1, 1), // TODO need check
    CalxInstr::LocalGet(_) => (0, 1),
    CalxInstr::LocalNew => (0, 0),
    CalxInstr::GlobalSet(_) => (1, 0),
    CalxInstr::GlobalGet(_) => (0, 1),
    CalxInstr::GlobalNew => (0, 0),
    CalxInstr::Const(_) => (0, 1),
    CalxInstr::Dup => (1, 2),
    CalxInstr::Drop => (1, 0),
    CalxInstr::IntAdd => (2, 1),
    CalxInstr::IntMul => (2, 1),
    CalxInstr::IntDiv => (2, 1),
    CalxInstr::IntRem => (2, 1),
    CalxInstr::IntNeg => (1, 1),
    CalxInstr::IntShr => (2, 1),
    CalxInstr::IntShl => (2, 1),
    CalxInstr::IntEq => (2, 1),
    CalxInstr::IntNe => (2, 1),
    CalxInstr::IntLt => (2, 1),
    CalxInstr::IntLe => (2, 1),
    CalxInstr::IntGt => (2, 1),
    CalxInstr::IntGe => (2, 1),
    CalxInstr::Add => (2, 1),
    CalxInstr::Mul => (2, 1),
    CalxInstr::Div => (2, 1),
    CalxInstr::Neg => (1, 1),
    // string operations
    // list operations
    CalxInstr::NewList => (0, 1),
    CalxInstr::ListGet => (2, 1),
    CalxInstr::ListSet => (3, 0),
    // Link
    CalxInstr::NewLink => (0, 1),
    // bool operations
    CalxInstr::And => (2, 1),
    CalxInstr::Or => (2, 1),
    CalxInstr::Not => (1, 1),
    // control stuctures
    CalxInstr::Br(_) => (0, 0),
    CalxInstr::BrIf(_) => (1, 0),
    CalxInstr::Jmp(_) => (0, 0),
    CalxInstr::JmpIf(_) => (1, 0),
    CalxInstr::Block { .. } => (0, 0),
    CalxInstr::BlockEnd(_) => (0, 0),
    CalxInstr::Echo => (1, 0),
    CalxInstr::Call(_) => (0, 0),       // TODO
    CalxInstr::ReturnCall(_) => (0, 0), // TODO
    CalxInstr::CallImport(_) => (0, 0), // import
    CalxInstr::Unreachable => (0, 0),   // TODO
    CalxInstr::Nop => (0, 0),
    CalxInstr::Quit(_) => (0, 0),
    CalxInstr::Return => (0, 0), // TODO
    CalxInstr::Assert(_) => (1, 0),
  }
}
