mod block_data;
pub mod frame;
pub mod func;
pub mod instr;

use std::collections::hash_map::HashMap;
use std::ops::Rem;
use std::rc::Rc;
use std::{fmt, vec};

use crate::calx::{Calx, CalxType};
use crate::syntax::CalxSyntax;
use crate::vm::block_data::BlockStack;

use self::block_data::BlockData;
use self::frame::CalxFrame;
use self::func::CalxFunc;
use self::instr::CalxInstr;

pub type CalxImportsDict = HashMap<String, (fn(xs: Vec<Calx>) -> Result<Calx, CalxError>, usize)>;

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
    let main_func = fns.iter().find(|x| *x.name == "main").expect("main function is required");
    let main_frame = CalxFrame {
      name: main_func.name.to_owned(),
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
      Some(f) => match f.instrs.to_owned() {
        Some(x) => x,
        None => return Err("main function must have instrs".to_owned()),
      },
      None => return Err("main function is required".to_owned()),
    };

    Ok(())
  }

  pub fn make_return(&mut self, v: Calx) {
    self.return_value = v;
    self.finished = true;
  }

  pub fn inspect_display(&self, indent_size: u8) -> String {
    let mut output = String::new();
    let indent = "\n".to_owned() + &" ".repeat(indent_size as usize);
    fmt::write(
      &mut output,
      format_args!(
        "{indent}Internal frames: {:?}",
        self.frames.iter().map(|x| x.name.to_owned()).collect::<Vec<_>>()
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
      self.check_func_return()?;
      if self.frames.is_empty() {
        let v = self.stack.pop().unwrap_or(Calx::Nil);
        self.make_return(v);
        return Ok(false);
      } else {
        // let prev_frame = self.top_frame;
        self.top_frame = self.frames.pop().unwrap();
      }
      self.top_frame.pointer += 1;
      return Ok(true);
    }
    let instrs = self.top_frame.instrs.to_owned();

    use instr::CalxInstr::*;

    match &instrs[self.top_frame.pointer] {
      Jmp(line) => {
        self.top_frame.pointer = line.to_owned();
        return Ok(true); // point reset, goto next loop
      }
      JmpOffset(l) => {
        self.top_frame.pointer = (self.top_frame.pointer as i32 + l) as usize;
        return Ok(true); // point reset, goto next loop
      }
      JmpIf(line) => {
        let v = self.stack_pop()?;
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          self.top_frame.pointer = line.to_owned();
          return Ok(true); // point reset, goto next loop
        }
      }
      JmpOffsetIf(l) => {
        let v = self.stack_pop()?;
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          self.top_frame.pointer = (self.top_frame.pointer as i32 + l) as usize;
          return Ok(true); // point reset, goto next loop
        }
      }
      LocalSet(idx) => {
        let v = self.stack_pop()?;
        if *idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.set {} for {:?}", idx, self.top_frame.locals)));
        } else {
          self.top_frame.locals[*idx] = v
        }
      }
      LocalTee(idx) => {
        let v = self.stack_pop()?;
        if *idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.tee {}", idx)));
        } else {
          self.top_frame.locals[*idx] = v.to_owned()
        }
        self.stack_push(v);
      }
      LocalGet(idx) => {
        if *idx < self.top_frame.locals.len() {
          self.stack_push(self.top_frame.locals[*idx].to_owned())
        } else {
          return Err(self.gen_err(format!("invalid index for local.get {}", idx)));
        }
      }
      Return => {
        // return values are moved to a temp space
        let mut ret_stack: Vec<Calx> = vec![];

        let ret_size = self.top_frame.ret_types.len();
        for _ in 0..ret_size {
          let v = self.stack_pop()?;
          ret_stack.insert(0, v);
        }

        self.check_func_return()?;

        if self.frames.is_empty() {
          // top frame return, just return value
          return match ret_stack.first() {
            Some(x) => {
              self.make_return(x.to_owned());
              Ok(false)
            }
            None => Err(self.gen_err("return without value".to_owned())),
          };
        } else {
          // let prev_frame = self.top_frame;
          self.top_frame = self.frames.pop().unwrap();
          // push return values back
          for v in ret_stack {
            self.stack_push(v);
          }
        }
      }
      LocalNew => self.top_frame.locals.push(Calx::Nil),
      GlobalSet(idx) => {
        let v = self.stack_pop()?;
        if self.globals.len() >= *idx {
          return Err(self.gen_err(format!("out of bound in global.set {}", idx)));
        } else {
          self.globals[*idx] = v
        }
      }
      GlobalGet(idx) => {
        if *idx < self.globals.len() {
          self.stack_push(self.globals[*idx].to_owned())
        } else {
          return Err(self.gen_err(format!("invalid index for global.get {}", idx)));
        }
      }
      GlobalNew => self.globals.push(Calx::Nil),
      Const(v) => self.stack_push(v.to_owned()),
      Dup => {
        self.stack_push(self.stack[self.stack.len() - 1].to_owned());
      }
      Drop => {
        let _ = self.stack_pop()?;
      }
      IntAdd => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&(self.stack[last_idx]), &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 + n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to add, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntMul => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 * n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to multiply, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntDiv => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 / n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to divide, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntRem => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64((*n1).rem(n2)),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to add, {:?} {:?}", self.stack[last_idx], v2))),
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
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 == n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to eq compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }

      IntNe => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 != n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to ne compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntLt => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 < n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to le compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntLe => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;
        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 <= n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to le compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntGt => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 > n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to gt compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      IntGe => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::Bool(n1 >= n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 integers to ge compare, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Add => {
        // reversed order
        let v2 = self.stack_pop()?;
        let last_idx = self.stack.len() - 1;

        match (&self.stack[last_idx], &v2) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[last_idx] = Calx::F64(n1 + n2),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[last_idx] = Calx::I64(n1 + n2),
          (_, _) => return Err(self.gen_err(format!("expected 2 numbers to +, {:?} {:?}", self.stack[last_idx], v2))),
        }
      }
      Mul => {
        // reversed order
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
      Call(f_name) => {
        // println!("frame size: {}", self.frames.len());
        match self.find_func(f_name) {
          Some(f) => {
            let instrs = f.instrs.to_owned();
            let ret_types = f.ret_types.to_owned();
            let f_name = f.name.to_owned();
            let mut locals: Vec<Calx> = vec![];
            for _ in 0..f.params_types.len() {
              let v = self.stack_pop()?;
              locals.insert(0, v);
            }
            self.frames.push(self.top_frame.to_owned());
            self.top_frame = CalxFrame {
              name: f_name,
              initial_stack_size: self.stack.len(),
              locals,
              pointer: 0,
              instrs: match instrs {
                Some(x) => x.to_owned(),
                None => unreachable!("function must have instrs"),
              },
              ret_types,
            };

            // start in new frame
            return Ok(true);
          }
          None => return Err(self.gen_err(format!("cannot find function named: {}", f_name))),
        }
      }
      ReturnCall(f_name) => {
        // println!("frame size: {}", self.frames.len());
        match self.find_func(f_name) {
          Some(f) => {
            // println!("examine stack: {:?}", self.stack);
            let instrs = f.instrs.to_owned();
            let ret_types = f.ret_types.to_owned();
            let f_name = f.name.to_owned();
            let mut locals: Vec<Calx> = vec![];
            for _ in 0..f.params_types.len() {
              let v = self.stack_pop()?;
              locals.insert(0, v);
            }
            let prev_frame = &self.top_frame;
            if prev_frame.initial_stack_size != self.stack.len() {
              return Err(self.gen_err(format!(
                "expected constant initial stack size: {}, got: {}",
                prev_frame.initial_stack_size,
                self.stack.len()
              )));
            }
            self.top_frame = CalxFrame {
              name: f_name,
              initial_stack_size: self.stack.len(),
              locals,
              pointer: 0,
              instrs: match instrs {
                Some(x) => x.to_owned(),
                None => panic!("function must have instrs"),
              },
              ret_types,
            };

            // start in new frame
            return Ok(true);
          }
          None => return Err(self.gen_err(format!("cannot find function named: {}", f_name))),
        }
      }
      CallImport(f_name) => match self.imports.to_owned().get(f_name) {
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
      Unreachable => {
        unreachable!("Unexpected from op")
      }
      Nop => {
        // Noop
      }
      Quit(code) => std::process::exit(*code as i32),
      Echo => {
        let v = self.stack_pop()?;
        println!("{}", v);
      }
      Assert(message) => {
        let v = self.stack_pop()?;
        if v == Calx::Bool(true) || v == Calx::I64(1) {
          // Ok
        } else {
          return Err(self.gen_err(format!("Failed assertion: {}", message)));
        }
      }
      Inspect => {
        println!("[ ----------------{}", self.inspect_display(2));
        println!("  -------------- ]");
      }
      If { ret_types, .. } => {
        // TODO
        self.check_stack_for_block(ret_types)?;
      }
      EndIf => {
        unreachable!("End if is internal instruction during preprocessing")
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
          println!("{} * {:?}", stack_size, self.funcs[i].syntax[j].to_owned());
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
              return Err(format!("insufficient params {} for block: {:?}", stack_size, params_types));
            }
            if *looped {
              blocks_track.push(BlockData::Loop {
                params_types: params_types.to_owned(),
                ret_types: ret_types.to_owned(),
                from: from.to_owned(),
                to: to.to_owned(),
                initial_stack_size: stack_size,
              });
            } else {
              blocks_track.push(BlockData::Block {
                params_types: params_types.to_owned(),
                ret_types: ret_types.to_owned(),
                to: to.to_owned(),
                initial_stack_size: stack_size,
              });
            }
            ops.push(CalxInstr::Nop);
          }
          CalxSyntax::Br(size) => {
            if *size > blocks_track.len() {
              return Err(format!("br {} too large", size));
            }

            let target_block = blocks_track.peek_block_level(*size)?;
            let expected_size = target_block.expected_finish_size();
            if stack_size != expected_size {
              return Err(format!("br({size}) expected size {expected_size}, got {stack_size}"));
            }

            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::Jmp(from.to_owned())),
              BlockData::Block { to, .. } => ops.push(CalxInstr::Jmp(to.to_owned())),
              _ => unreachable!("br target must be block or loop"),
            }
          }
          CalxSyntax::BrIf(size) => {
            if blocks_track.is_empty() {
              return Err(format!("cannot branch with no blocks, {}", size));
            }
            if *size > blocks_track.len() {
              return Err(format!("br {} too large", size));
            }

            let target_block = blocks_track.peek_block_level(*size)?;

            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::JmpIf(from.to_owned())),
              BlockData::Block { to, .. } => ops.push(CalxInstr::JmpIf(to.to_owned())),
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
              return Err(format!("invalid block end, {:?}", blocks_track));
            }

            let prev_block = blocks_track.pop_block()?;
            if *looped {
              // nothing, branched during runtime
            } else if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for block end: {} {:?}", stack_size, prev_block));
            }

            ops.push(CalxInstr::Nop)
          }
          CalxSyntax::Call(f_name) => match self.find_func(f_name) {
            Some(f) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::Call(f_name.to_owned()))
            }
            None => return Err(format!("cannot find function named: {}", f_name)),
          },
          CalxSyntax::ReturnCall(f_name) => match self.find_func(f_name) {
            Some(f) => {
              if stack_size < f.params_types.len() {
                return Err(format!("insufficient size to call: {} {:?}", stack_size, f.params_types));
              }
              stack_size = stack_size - f.params_types.len() + f.ret_types.len();
              ops.push(CalxInstr::ReturnCall(f_name.to_owned()))
            }
            None => return Err(format!("cannot find function named: {}", f_name)),
          },
          CalxSyntax::CallImport(f_name) => match &self.imports.get(f_name) {
            Some((_f, size)) => {
              if stack_size < *size {
                return Err(format!("insufficient size to call import: {} {:?}", stack_size, size));
              }
              stack_size = stack_size - size + 1;
              ops.push(CalxInstr::CallImport(f_name.to_owned()))
            }
            None => return Err(format!("missing imported function {}", f_name)),
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
              return Err(format!("insufficient stack {} to branch", stack_size));
            }

            blocks_track.push(BlockData::If {
              ret_types: ret_types.to_owned(),
              else_to: else_at.to_owned(),
              to: to.to_owned(),
              initial_stack_size: stack_size,
            });

            stack_size -= 1;
            ops.push(CalxInstr::JmpIf(else_at.to_owned()));
          }
          CalxSyntax::ElseEnd => {
            if blocks_track.is_empty() {
              return Err(format!("invalid else end, {:?}", blocks_track));
            }

            let prev_block = blocks_track.peek_if()?;

            if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for else-end: {} {:?}", stack_size, prev_block));
            }

            match prev_block {
              BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(to.to_owned())),
              _ => unreachable!("end inside if"),
            }
          }
          CalxSyntax::ThenEnd => {
            if blocks_track.is_empty() {
              return Err(format!("invalid else end, {:?}", blocks_track));
            }

            let prev_block = blocks_track.pop_if()?;
            if stack_size != prev_block.expected_finish_size() {
              return Err(format!("size mismatch for then-end: {} {:?}", stack_size, prev_block));
            }

            match prev_block {
              BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(to.to_owned())),
              _ => unreachable!("end inside if"),
            }
          }
          a => {
            let instr: CalxInstr = a.try_into()?;
            // checks
            let (params_size, ret_size) = instr.stack_arity();
            if stack_size < params_size {
              return Err(format!("insufficient stack {} to call {:?} of {}", stack_size, a, params_size));
            }
            stack_size = stack_size - params_size + ret_size;
            // println!(
            //   "  sizes: {:?} {} {} -> {}",
            //   a, params_size, ret_size, stack_size
            // );
            ops.push(instr.to_owned());
          }
        }
      }
      if stack_size != 0 {
        return Err(format!(
          "invalid final size {} of {:?} in {}",
          stack_size, self.funcs[i].ret_types, self.funcs[i].name
        ));
      }

      self.funcs[i].instrs = Some(Rc::new(ops));
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
  fn check_func_return(&self) -> Result<(), CalxError> {
    if self.stack.len() != self.top_frame.initial_stack_size {
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

  fn gen_err(&self, s: String) -> CalxError {
    CalxError {
      message: s,
      top_frame: self.top_frame.to_owned(),
      stack: self.stack.to_owned(),
      globals: self.globals.to_owned(),
    }
  }

  fn find_func(&self, name: &str) -> Option<&CalxFunc> {
    self.funcs.iter().find(|x| *x.name == name)
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
