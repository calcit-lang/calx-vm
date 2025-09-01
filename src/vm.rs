mod block_data;
pub mod frame;
pub mod func;
pub mod instr;

use std::collections::hash_map::HashMap;
use std::ops::Rem;
use std::rc::Rc;
use std::{fmt, mem, vec};

use crate::calx::Calx;
use crate::syntax::CalxSyntax;
use crate::vm::block_data::BlockStack;

use self::block_data::BlockData;
use self::frame::CalxFrame;
use self::func::CalxFunc;
use self::instr::CalxInstr;

pub type CalxImportsDict = HashMap<Rc<str>, (fn(xs: &Vec<Calx>) -> Result<Calx, CalxError>, usize)>;

/// Virtual Machine for Calx
/// code is evaluated in a several steps:
/// 1. parse into `CalxSyntax`
/// 2. preprocess `CalxSyntax` into instructions(`CalxInstr`)
/// 3. run instructions
///
/// `CalxSyntax` contains some richer info than `CalxInstr`.
#[derive(Clone)]
pub struct CalxVM {
  pub stack: Vec<Calx>,
  pub globals: Vec<Calx>,
  pub funcs: Vec<CalxFunc>,
  pub frames: Vec<CalxFrame>,
  pub top_frame: CalxFrame,
  pub imports: CalxImportsDict,
  /// extra status to tracking runnnig finished
  pub finished: bool,
  pub return_value: Calx,
}

impl std::fmt::Debug for CalxVM {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("CalxVM Instance")
  }
}

impl CalxVM {
  pub fn new(fns: Vec<CalxFunc>, globals: Vec<Calx>, imports: CalxImportsDict) -> Self {
    let main_func = fns.iter().find(|x| &*x.name == "main").expect("main function is required");
    let main_frame = CalxFrame {
      name: main_func.name.clone(),
      initial_stack_size: 0,
      // use empty instrs, will be replaced by preprocess
      instrs: Rc::new(vec![]),
      pointer: 0,
      locals: vec![],
      ret_types: main_func.ret_types.clone(),
    };
    CalxVM {
      stack: vec![],
      globals,
      funcs: fns,
      frames: vec![],
      top_frame: main_frame,
      imports,
      return_value: Calx::Nil,
      finished: false,
    }
  }

  pub fn setup_top_frame(&mut self) -> Result<(), String> {
    self.top_frame.instrs = match self.find_func("main") {
      Some(f) => f.instrs.to_owned(),
      None => return Err("main function is required".to_string()),
    };

    Ok(())
  }

  pub fn make_return(&mut self, v: Calx) {
    self.return_value = v;
    self.finished = true;
  }

  pub fn inspect_display(&self, indent_size: u8) -> String {
    let mut output = String::new();
    let indent = "\n".to_string() + &" ".repeat(indent_size as usize);
    fmt::write(
      &mut output,
      format_args!(
        "{indent}Internal frames: {:?}",
        self.frames.iter().map(|x| &*x.name).collect::<Vec<_>>()
      ),
    )
    .expect("inspect display");

    fmt::write(&mut output, format_args!("{indent}Top frame: {}", self.top_frame.name)).expect("inspect display");
    fmt::write(&mut output, format_args!("{indent}Locals: {:?}", self.top_frame.locals)).expect("inspect display");
    fmt::write(&mut output, format_args!("{indent}Stack({}): {:?}", self.stack.len(), self.stack)).expect("inspect display");
    fmt::write(
      &mut output,
      format_args!(
        "{indent}Sizes: {} + {}",
        self.top_frame.initial_stack_size,
        self.top_frame.ret_types.len()
      ),
    )
    .expect("inspect display");
    fmt::write(&mut output, format_args!("{indent}Pointer: {}", self.top_frame.pointer)).expect("inspect display");
    output
  }

  pub fn run(&mut self, args: Vec<Calx>) -> Result<Calx, CalxError> {
    // assign function parameters
    self.top_frame.locals = args;
    self.stack.clear();
    loop {
      // println!("Stack {:?}", self.stack);
      // println!("-- op {} {:?}", self.stack.len(), instr);

      if self.finished {
        return Ok(self.return_value.to_owned());
      }

      let quick_continue = self.step()?;
      if quick_continue {
        continue;
      }

      self.top_frame.pointer += 1;
    }
  }

  /// run one step, return true if continuing
  #[inline(always)]
  pub fn step(&mut self) -> Result<bool, CalxError> {
    if self.top_frame.pointer >= self.top_frame.instrs.len() {
      // println!("status {:?} {}", self.stack, self.top_frame);
      self.check_func_return(self.top_frame.ret_types.len())?;

      match self.frames.pop() {
        Some(v) => {
          self.top_frame = v;
        }
        None => {
          let v = self.stack.pop().unwrap_or(Calx::Nil);
          self.make_return(v);
          return Ok(false);
        }
      }

      self.top_frame.pointer += 1;
      return Ok(true);
    }
    let instr = &self.top_frame.instrs[self.top_frame.pointer];

    use instr::CalxInstr::*;

    match instr {
      Jmp(line) => {
        self.top_frame.pointer = *line;
        return Ok(true); // point reset, goto next loop
      }
      JmpOffset(l) => {
        self.top_frame.pointer = (self.top_frame.pointer as i32 + l) as usize;
        return Ok(true); // point reset, goto next loop
      }
      JmpIf(line) => {
        let v = self.stack.pop().unwrap();
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          self.top_frame.pointer = *line;
          return Ok(true); // point reset, goto next loop
        }
      }
      JmpOffsetIf(l) => {
        self.check_before_pop()?;
        let v = self.stack.pop().expect("pop value");
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          self.top_frame.pointer = (self.top_frame.pointer as i32 + l) as usize;
          return Ok(true); // point reset, goto next loop
        }
      }
      LocalSet(idx) => {
        self.check_before_pop()?;
        let v = self.stack.pop().expect("pop value");
        if *idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.set {} for {:?}", idx, self.top_frame.locals)));
        } else {
          self.top_frame.locals[*idx] = v
        }
      }
      LocalTee(idx) => {
        self.check_before_pop()?;
        let v = self.stack.pop().expect("pop value");
        if *idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.tee {idx}")));
        } else {
          v.clone_into(&mut self.top_frame.locals[*idx])
        }
        self.stack_push(v);
      }
      LocalGet(idx) => {
        if *idx < self.top_frame.locals.len() {
          // 优化：对Copy类型避免clone
          let local_val = &self.top_frame.locals[*idx];
          match local_val {
            Calx::I64(n) => self.stack.push(Calx::I64(*n)),
            Calx::F64(n) => self.stack.push(Calx::F64(*n)),
            Calx::Bool(b) => self.stack.push(Calx::Bool(*b)),
            Calx::Nil => self.stack.push(Calx::Nil),
            _ => self.stack.push(local_val.clone()),
          }
        } else {
          return Err(self.gen_err(format!("invalid index for local.get {idx}")));
        }
      }
      Return => {
        // return values are moved to a temp space

        let ret_size = self.top_frame.ret_types.len();

        self.check_func_return(ret_size)?;

        if self.frames.is_empty() {
          // top frame return, just return value
          return match self.stack.last() {
            Some(x) => {
              self.make_return(x.to_owned());
              Ok(false)
            }
            None => Err(self.gen_err("return without value".to_string())),
          };
        } else {
          // let prev_frame = self.top_frame;
          self.top_frame = self.frames.pop().unwrap();
        }
      }
      LocalNew => self.top_frame.locals.push(Calx::Nil),
      GlobalSet(idx) => {
        self.check_before_pop()?;
        let v = self.stack.pop().expect("pop value");
        if self.globals.len() >= *idx {
          return Err(self.gen_err(format!("out of bound in global.set {idx}")));
        } else {
          self.globals[*idx] = v
        }
      }
      GlobalGet(idx) => {
        if *idx < self.globals.len() {
          self.stack_push(self.globals[*idx].to_owned())
        } else {
          return Err(self.gen_err(format!("invalid index for global.get {idx}")));
        }
      }
      GlobalNew => self.globals.push(Calx::Nil),
      Const(v) => {
        // 优化：对Copy类型避免clone
        match v {
          Calx::I64(n) => self.stack.push(Calx::I64(*n)),
          Calx::F64(n) => self.stack.push(Calx::F64(*n)),
          Calx::Bool(b) => self.stack.push(Calx::Bool(*b)),
          Calx::Nil => self.stack.push(Calx::Nil),
          _ => self.stack.push(v.clone()),
        }
      }
      Dup => {
        // 优化：避免不必要的clone，对于Copy类型直接复制
        if let Some(last) = self.stack.last() {
          match last {
            Calx::I64(n) => self.stack.push(Calx::I64(*n)),
            Calx::F64(n) => self.stack.push(Calx::F64(*n)),
            Calx::Bool(b) => self.stack.push(Calx::Bool(*b)),
            Calx::Nil => self.stack.push(Calx::Nil),
            _ => self.stack.push(last.clone()),
          }
        }
      }
      Drop => {
        let _ = self.stack_pop()?;
      }
      IntAdd => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::I64(n1 + n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to add, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntMul => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::I64(n1 * n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to multiply, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntDiv => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::I64(n1 / n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to divide, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntRem => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::I64((*n1).rem(n2));
        } else {
          return Err(self.gen_err(format!("expected 2 integers for remainder, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntNeg => {
        let last_idx = self.stack.len() - 1;
        if let Calx::I64(n) = self.stack[last_idx] {
          self.stack[last_idx] = Calx::I64(-n)
        } else {
          return Err(self.gen_err(format!("expected int, got {}", self.stack[last_idx])));
        }
      }
      IntShr => {
        let bits = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &bits) {
          (Calx::I64(n), Calx::I64(b)) => self.stack[last_idx] = Calx::I64(n.checked_shr(*b as u32).unwrap()),
          (_, _) => return Err(self.gen_err(format!("invalid number for SHR, {:?} {:?}", self.stack[last_idx], bits))),
        }
      }
      IntShl => {
        let bits = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &bits) {
          (Calx::I64(n), Calx::I64(b)) => self.stack[last_idx] = Calx::I64(n.checked_shl(*b as u32).unwrap()),
          (_, _) => return Err(self.gen_err(format!("invalid number for SHL, {:?} {:?}", self.stack[last_idx], bits))),
        }
      }
      IntEq => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::Bool(n1 == n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to eq compare, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }

      IntNe => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 != n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to ne compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntLt => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::Bool(n1 < n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to lt compare, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntLe => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::Bool(n1 <= n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to le compare, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntGt => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        if let (Calx::I64(n1), Calx::I64(n2)) = (&self.stack[last_idx], &v2) {
          self.stack[last_idx] = Calx::Bool(n1 > n2);
        } else {
          return Err(self.gen_err(format!("expected 2 integers to gt compare, {:?} {:?}", self.stack[last_idx], v2)));
        }
      }
      IntGe => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 >= n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to ge compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Add => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 + n2),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 + n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 numbers to +, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Mul => {
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 * n2),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 * n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 numbers to multiply, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Div => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 / n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 numbers to divide, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Neg => {
        let last_idx = self.stack.len() - 1;
        if let Calx::F64(n) = self.stack[last_idx] {
          self.stack[last_idx] = Calx::F64(-n)
        } else {
          return Err(self.gen_err(format!("expected float, got {}", self.stack[last_idx])));
        }
      }
      NewList => {
        todo!()
      }
      ListGet => {
        todo!()
      }
      ListSet => {
        todo!()
      }
      NewLink => {
        todo!()
      }
      And => {
        todo!()
      }
      Or => {
        todo!()
      }
      Not => {
        todo!()
      }
      Call(idx) => {
        // println!("frame size: {}", self.frames.len());
        let f = &self.funcs[*idx];
        let instrs = &f.instrs;
        let ret_types = f.ret_types.clone();
        let f_name = f.name.clone();

        let n = f.params_types.len();
        self.check_before_pop_n(n)?;
        let next_size = self.stack.len() - n;
        let locals = self.stack.split_off(next_size);

        // TODO reduce copy drop

        let new_frame = CalxFrame {
          name: f_name,
          initial_stack_size: next_size,
          locals,
          pointer: 0,
          instrs: instrs.to_owned(),
          ret_types,
        };
        let prev_frame = mem::replace(&mut self.top_frame, new_frame);
        self.frames.push(prev_frame);

        // start in new frame
        return Ok(true);
      }
      ReturnCall(idx) => {
        // println!("frame size: {}", self.frames.len());
        let f = &self.funcs[*idx];

        // println!("examine stack: {:?}", self.stack);
        let instrs = &f.instrs;
        let ret_types = f.ret_types.clone();
        let f_name = f.name.clone();

        let n = f.params_types.len();
        self.check_before_pop_n(n)?;

        let next_size = self.stack.len() - n;
        let locals = self.stack.split_off(next_size);

        let prev_frame = &self.top_frame;
        if prev_frame.initial_stack_size != next_size {
          return Err(self.gen_err(format!(
            "expected constant initial stack size: {}, got: {}",
            prev_frame.initial_stack_size,
            self.stack.len()
          )));
        }
        self.top_frame = CalxFrame {
          name: f_name,
          initial_stack_size: next_size,
          locals,
          pointer: 0,
          instrs: instrs.to_owned(),
          ret_types,
        };

        // start in new frame
        return Ok(true);
      }
      CallImport(f_name) => match self.imports.get(f_name) {
        None => return Err(self.gen_err(format!("missing imported function {f_name}"))),
        Some((f, size)) => {
          if self.stack.len() < *size {
            return Err(self.gen_err(format!(
              "imported function {} expected {} arguemtns, found {} on stack",
              f_name,
              size,
              self.stack.len()
            )));
          }

          let n = *size;
          self.check_before_pop_n(n)?;
          let args = self.stack.split_off(self.stack.len() - n);

          let v = f(&args)?;
          self.stack_push(v);
        }
      },
      Unreachable => {
        unreachable!("Unexpected from op")
      }
      Nop => {
        // Noop
      }
      Quit(code) => std::process::exit(*code as i32),
      Echo => {
        let v = self.stack_pop()?;
        println!("{v}");
      }
      Assert(message) => {
        self.check_before_pop()?;
        let v = self.stack.pop().expect("pop value");
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          // Ok
        } else {
          return Err(self.gen_err(format!("Failed assertion: {message}")));
        }
      }
      Inspect => {
        println!("[ ----------------{}", self.inspect_display(2));
        println!("  -------------- ]");
      }
    }

    Ok(false)
  }

  pub fn preprocess(&mut self, verbose: bool) -> Result<(), String> {
    for i in 0..self.funcs.len() {
      let mut stack_size = 0;
      let mut ops: Vec<CalxInstr> = vec![];
      let mut blocks_track = BlockStack::new();

      let f = &self.funcs[i];

      if verbose {
        println!(
          "\nFUNC {}\n  initial stack size: {}\n  ret_size {}",
          f.name,
          stack_size,
          f.ret_types.len()
        );
      }

      for j in 0..self.funcs[i].syntax.len() {
        if verbose {
          println!("{} * {:?}", stack_size, self.funcs[i].syntax[j]);
        }
        let syntax = &self.funcs[i].syntax;
        match &syntax[j] {
          CalxSyntax::Block {
            looped,
            params_types,
            ret_types,
            from,
            to,
          } => {
            if stack_size < params_types.len() {
              return Err(format!("insufficient params {stack_size} for block: {params_types:?}"));
            }
            if *looped {
              blocks_track.push(BlockData::Loop {
                params_types: params_types.clone(),
                ret_types: ret_types.clone(),
                from: *from,
                to: *to,
                initial_stack_size: stack_size,
              });
            } else {
              blocks_track.push(BlockData::Block {
                params_types: params_types.clone(),
                ret_types: ret_types.clone(),
                to: *to,
                initial_stack_size: stack_size,
              });
            }
            ops.push(CalxInstr::Nop);
          }
          CalxSyntax::Br(size) => {
            if *size > blocks_track.len() {
              return Err(format!("br {size} too large"));
            }

            let target_block = blocks_track.peek_block_level(*size)?;
            let expected_size = target_block.expected_finish_size();
            if stack_size != expected_size {
              return Err(format!("br({size}) expected size {expected_size}, got {stack_size}"));
            }

            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::Jmp(*from)),
              BlockData::Block { to, .. } => ops.push(CalxInstr::Jmp(*to)),
              _ => unreachable!("br target must be block or loop"),
            }
          }
          CalxSyntax::BrIf(size) => {
            if blocks_track.is_empty() {
              return Err(format!("cannot branch with no blocks, {size}"));
            }
            if *size > blocks_track.len() {
              return Err(format!("br {size} too large"));
            }

            let target_block = blocks_track.peek_block_level(*size)?;

            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::JmpIf(*from)),
              BlockData::Block { to, .. } => ops.push(CalxInstr::JmpIf(*to)),
              _ => unreachable!("br target must be block or loop"),
            }
            stack_size -= 1;

            let expected_size = target_block.expected_finish_size();
            if stack_size != expected_size {
              return Err(format!("brIf({size}) expected size {expected_size}, got {stack_size}"));
            }
          }
          CalxSyntax::BlockEnd(looped) => {
            // println!("checking: {:?}", blocks_track);
            if blocks_track.is_empty() {
              return Err(format!("invalid block end, {blocks_track:?}"));
            }

            let prev_block = blocks_track.pop_block()?;
            if *looped {
              // nothing, branched during runtime
            } else if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for block end: {stack_size} {prev_block:?}"));
            }

            ops.push(CalxInstr::Nop)
          }
          CalxSyntax::Call(f_name) => match self.find_func_idx(f_name) {
            Some((idx, f)) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::Call(idx));
            }
            None => return Err(format!("cannot find function named: {f_name}")),
          },
          CalxSyntax::ReturnCall(f_name) => match self.find_func_idx(f_name) {
            Some((idx, f)) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::ReturnCall(idx))
            }
            None => return Err(format!("cannot find function named: {f_name}")),
          },
          CalxSyntax::CallImport(f_name) => match &self.imports.get(f_name) {
            Some((_f, size)) => {
              if stack_size < *size {
                return Err(format!("insufficient size to call import: {stack_size} {size:?}"));
              }
              stack_size = stack_size - size + 1;
              ops.push(CalxInstr::CallImport(f_name.to_owned()))
            }
            None => return Err(format!("missing imported function {f_name}")),
          },
          CalxSyntax::Return => {
            let ret_size = self.funcs[i].ret_types.len();
            stack_size -= ret_size;
            if stack_size != 0 {
              return Err(format!(
                "invalid return size {} for {:?} in {}",
                stack_size, self.funcs[i].ret_types, self.funcs[i].name
              ));
            }
            ops.push(CalxInstr::Return);
          }
          CalxSyntax::If { ret_types, else_at, to } => {
            if stack_size < 1 {
              return Err(format!("insufficient stack {stack_size} to branch"));
            }

            blocks_track.push(BlockData::If {
              ret_types: ret_types.clone(),
              else_to: *else_at,
              to: *to,
              initial_stack_size: stack_size,
            });

            stack_size -= 1;
            ops.push(CalxInstr::JmpIf(*else_at));
          }
          CalxSyntax::ElseEnd => {
            if blocks_track.is_empty() {
              return Err(format!("invalid else end, {blocks_track:?}"));
            }

            let prev_block = blocks_track.peek_if()?;

            if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for else-end: {stack_size} {prev_block:?}"));
            }

            match prev_block {
              BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(*to)),
              _ => unreachable!("end inside if"),
            }
          }
          CalxSyntax::ThenEnd => {
            if blocks_track.is_empty() {
              return Err(format!("invalid else end, {blocks_track:?}"));
            }

            let prev_block = blocks_track.pop_if()?;
            if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for then-end: {stack_size} {prev_block:?}"));
            }

            match prev_block {
              BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(to)),
              _ => unreachable!("end inside if"),
            }
          }
          a => {
            let instr: CalxInstr = a.try_into()?;
            // checks
            let (params_size, ret_size) = instr.stack_arity();
            if stack_size < params_size {
              return Err(format!("insufficient stack {stack_size} to call {a:?} of {params_size}"));
            }
            stack_size = stack_size - params_size + ret_size;
            // println!(
            //   "  sizes: {:?} {} {} -> {}",
            //   a, params_size, ret_size, stack_size
            // );
            ops.push(instr);
          }
        }
      }
      if stack_size != 0 {
        return Err(format!(
          "invalid final size {} of {:?} in {}",
          stack_size, self.funcs[i].ret_types, self.funcs[i].name
        ));
      }

      self.funcs[i].instrs = Rc::new(ops);
    }

    Ok(())
  }

  #[inline(always)]
  fn check_func_return(&self, ret_size: usize) -> Result<(), CalxError> {
    if self.stack.len() - ret_size != self.top_frame.initial_stack_size {
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
    let stack_len = self.stack.len();
    if stack_len <= self.top_frame.initial_stack_size {
      Err(self.gen_err(String::from("cannot pop from parent stack")))
    } else {
      // Optimization: use unsafe to avoid repeated bounds checking
      Ok(unsafe { self.stack.pop().unwrap_unchecked() })
    }
  }

  fn check_before_pop(&self) -> Result<(), CalxError> {
    if self.stack.is_empty() {
      return Err(self.gen_err(String::from("cannot pop from empty stack")));
    } else if self.stack.len() <= self.top_frame.initial_stack_size {
      return Err(self.gen_err(String::from("cannot pop from parent stack")));
    }
    Ok(())
  }

  fn check_before_pop_n(&self, n: usize) -> Result<(), CalxError> {
    if self.stack.len() < n {
      return Err(self.gen_err(String::from("cannot pop from empty stack")));
    } else if self.stack.len() - n < self.top_frame.initial_stack_size {
      return Err(self.gen_err(String::from("cannot pop from parent stack")));
    }
    Ok(())
  }

  #[inline(always)]
  fn stack_push(&mut self, x: Calx) {
    self.stack.push(x)
  }

  fn gen_err(&self, s: String) -> CalxError {
    CalxError {
      message: s,
      top_frame: self.top_frame.to_owned(),
      stack: self.stack.to_owned(),
      globals: self.globals.to_owned(),
    }
  }

  fn find_func(&self, name: &str) -> Option<&CalxFunc> {
    self.funcs.iter().find(|x| &*x.name == name)
  }

  fn find_func_idx(&self, name: &str) -> Option<(usize, &CalxFunc)> {
    self.funcs.iter().enumerate().find(|pair| &*pair.1.name == name)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxError {
  pub message: String,
  pub stack: Vec<Calx>,
  pub top_frame: CalxFrame,
  pub globals: Vec<Calx>,
}

impl fmt::Display for CalxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}\n{:?}\n{}", self.message, self.stack, self.top_frame)
  }
}

impl CalxError {
  pub fn new_raw(s: String) -> Self {
    CalxError {
      message: s,
      stack: vec![],
      top_frame: CalxFrame::default(),
      globals: vec![],
    }
  }
}
