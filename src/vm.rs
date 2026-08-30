mod block_data;
pub mod frame;
pub mod func;
pub mod instr;

use std::collections::hash_map::HashMap;
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
    let main_frame = match fns.iter().find(|x| &*x.name == "main") {
      Some(main_func) => CalxFrame {
        name: main_func.name.clone(),
        initial_stack_size: 0,
        // use empty instrs, will be replaced by preprocess
        instrs: Rc::new(vec![]),
        pointer: 0,
        locals: vec![],
        ret_types: main_func.ret_types.clone(),
      },
      None => CalxFrame::default(),
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
    output.push_str(&format!(
      "{indent}Internal frames: {:?}",
      self.frames.iter().map(|x| &*x.name).collect::<Vec<_>>()
    ));
    output.push_str(&format!("{indent}Top frame: {}", self.top_frame.name));
    output.push_str(&format!("{indent}Locals: {:?}", self.top_frame.locals));
    output.push_str(&format!("{indent}Stack({}): {:?}", self.stack.len(), self.stack));
    output.push_str(&format!(
      "{indent}Sizes: {} + {}",
      self.top_frame.initial_stack_size,
      self.top_frame.ret_types.len()
    ));
    output.push_str(&format!("{indent}Pointer: {}", self.top_frame.pointer));
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
      self.collect_return_values(self.top_frame.ret_types.len())?;

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
      Branch { target, base, arity } => {
        let (target, base, arity) = (*target, *base, *arity);
        self.apply_branch_stack(base, arity)?;
        self.top_frame.pointer = target;
        return Ok(true);
      }
      JmpOffset(l) => {
        self.top_frame.pointer = self.pointer_with_offset(*l)?;
        return Ok(true); // point reset, goto next loop
      }
      JmpIf(line) => {
        let line = *line;
        let v = self.stack_pop()?;
        if v.truthy() {
          self.top_frame.pointer = line;
          return Ok(true); // point reset, goto next loop
        }
      }
      BranchIf { target, base, arity } => {
        let (target, base, arity) = (*target, *base, *arity);
        let condition = self.stack_pop()?;
        if condition.truthy() {
          self.apply_branch_stack(base, arity)?;
          self.top_frame.pointer = target;
          return Ok(true);
        }
      }
      JmpOffsetIf(l) => {
        let offset = *l;
        let v = self.stack_pop()?;
        if v.truthy() {
          self.top_frame.pointer = self.pointer_with_offset(offset)?;
          return Ok(true); // point reset, goto next loop
        }
      }
      LocalSet(idx) => {
        let idx = *idx;
        let v = self.stack_pop()?;
        if idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.set {} for {:?}", idx, self.top_frame.locals)));
        } else {
          self.top_frame.locals[idx] = v
        }
      }
      LocalTee(idx) => {
        let idx = *idx;
        let v = self.stack_pop()?;
        if idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.tee {idx}")));
        } else {
          v.clone_into(&mut self.top_frame.locals[idx])
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

        self.collect_return_values(ret_size)?;

        if self.frames.is_empty() {
          // top frame return, just return value
          return match (ret_size, self.stack.last()) {
            (0, _) => {
              self.make_return(Calx::Nil);
              Ok(false)
            }
            (_, Some(x)) => {
              self.make_return(x.to_owned());
              Ok(false)
            }
            (_, None) => Err(self.gen_err("return without value".to_string())),
          };
        } else {
          let Some(previous_frame) = self.frames.pop() else {
            return Err(self.gen_err("missing caller frame while returning".to_string()));
          };
          self.top_frame = previous_frame;
        }
      }
      LocalNew => self.top_frame.locals.push(Calx::Nil),
      GlobalSet(idx) => {
        let idx = *idx;
        let v = self.stack_pop()?;
        if idx >= self.globals.len() {
          return Err(self.gen_err(format!("out of bound in global.set {idx}")));
        } else {
          self.globals[idx] = v
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
        self.check_before_pop()?;
        let value = self
          .stack
          .last()
          .cloned()
          .ok_or_else(|| self.gen_err("cannot duplicate an empty stack".to_string()))?;
        self.stack_push(value);
      }
      Drop => {
        let _ = self.stack_pop()?;
      }
      IntAdd => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1.wrapping_add(*n2)),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to add, {v1:?} {v2:?}"))),
        }
      }
      IntMul => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1.wrapping_mul(*n2)),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to multiply, {v1:?} {v2:?}"))),
        }
      }
      IntDiv => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(_), Calx::I64(0)) => return Err(self.gen_err("trap: integer divide by zero".to_string())),
          (Calx::I64(n1), Calx::I64(n2)) if *n1 == i64::MIN && *n2 == -1 => {
            return Err(self.gen_err("trap: integer division overflow".to_string()));
          }
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1 / n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to divide, {v1:?} {v2:?}"))),
        }
      }
      IntRem => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(_), Calx::I64(0)) => return Err(self.gen_err("trap: integer remainder by zero".to_string())),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1.wrapping_rem(*n2)),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers for remainder, {v1:?} {v2:?}"))),
        }
      }
      IntNeg => {
        self.check_before_pop()?;
        let top = self.stack.len() - 1;
        match self.stack[top] {
          Calx::I64(n) => self.stack[top] = Calx::I64(n.wrapping_neg()),
          ref value => return Err(self.gen_err(format!("expected int, got {value}"))),
        }
      }
      IntShr => {
        let (left, bits) = self.stack_pop_right()?;
        match (&self.stack[left], &bits) {
          (Calx::I64(n), Calx::I64(b)) => self.stack[left] = Calx::I64(n.wrapping_shr(*b as u32)),
          (value, bits) => return Err(self.gen_err(format!("invalid number for SHR, {value:?} {bits:?}"))),
        }
      }
      IntShl => {
        let (left, bits) = self.stack_pop_right()?;
        match (&self.stack[left], &bits) {
          (Calx::I64(n), Calx::I64(b)) => self.stack[left] = Calx::I64(n.wrapping_shl(*b as u32)),
          (value, bits) => return Err(self.gen_err(format!("invalid number for SHL, {value:?} {bits:?}"))),
        }
      }
      IntEq => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 == n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to eq compare, {v1:?} {v2:?}"))),
        }
      }

      IntNe => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 != n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to ne compare, {v1:?} {v2:?}"))),
        }
      }
      IntLt => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 < n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to lt compare, {v1:?} {v2:?}"))),
        }
      }
      IntLe => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 <= n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to le compare, {v1:?} {v2:?}"))),
        }
      }
      IntGt => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 > n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to gt compare, {v1:?} {v2:?}"))),
        }
      }
      IntGe => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::Bool(n1 >= n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 integers to ge compare, {v1:?} {v2:?}"))),
        }
      }
      Add => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[left] = Calx::F64(n1 + n2),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1.wrapping_add(*n2)),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 numbers to add, {v1:?} {v2:?}"))),
        }
      }
      Mul => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[left] = Calx::F64(n1 * n2),
          (Calx::I64(n1), Calx::I64(n2)) => self.stack[left] = Calx::I64(n1.wrapping_mul(*n2)),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 numbers to multiply, {v1:?} {v2:?}"))),
        }
      }
      Div => {
        let (left, right) = self.stack_pop_right()?;
        match (&self.stack[left], &right) {
          (Calx::F64(n1), Calx::F64(n2)) => self.stack[left] = Calx::F64(n1 / n2),
          (v1, v2) => return Err(self.gen_err(format!("expected 2 floats to divide, {v1:?} {v2:?}"))),
        }
      }
      Neg => {
        self.check_before_pop()?;
        let top = self.stack.len() - 1;
        match self.stack[top] {
          Calx::F64(n) => self.stack[top] = Calx::F64(-n),
          ref value => return Err(self.gen_err(format!("expected float, got {value}"))),
        }
      }
      NewList | ListGet | ListSet | NewLink | And | Or | Not => {
        return Err(self.gen_err(format!("unsupported instruction reached execution: {instr:?}")));
      }
      Call(idx) => {
        // println!("frame size: {}", self.frames.len());
        let Some(f) = self.funcs.get(*idx) else {
          return Err(self.gen_err(format!("invalid function index for call: {idx}")));
        };
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
        let Some(f) = self.funcs.get(*idx) else {
          return Err(self.gen_err(format!("invalid function index for return-call: {idx}")));
        };

        // println!("examine stack: {:?}", self.stack);
        let instrs = &f.instrs;
        let ret_types = f.ret_types.clone();
        let f_name = f.name.clone();

        let n = f.params_types.len();
        self.check_before_pop_n(n)?;

        let args_at = self.stack.len() - n;
        let locals = self.stack.split_off(args_at);
        let next_size = self.top_frame.initial_stack_size;
        self.stack.truncate(next_size);
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
        return Err(self.gen_err("trap: unreachable instruction executed".to_string()));
      }
      Nop => {
        // Noop
      }
      Quit(code) => {
        return Err(self.gen_err(format!("trap: guest requested process exit with status {code}")));
      }
      Echo => {
        let v = self.stack_pop()?;
        println!("{v}");
      }
      Assert(message) => {
        let message = message.clone();
        let v = self.stack_pop()?;
        if v.truthy() {
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
    crate::validate_program(&self.funcs, &self.globals, &self.imports).map_err(|e| e.to_string())?;

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
            let target_block = blocks_track.peek_block_level(*size)?;
            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::Branch {
                target: *from,
                base: target_block.branch_base(),
                arity: target_block.branch_arity(),
              }),
              BlockData::Block { to, .. } => ops.push(CalxInstr::Branch {
                target: *to,
                base: target_block.branch_base(),
                arity: target_block.branch_arity(),
              }),
              BlockData::If { .. } => return Err("br target must be block or loop".to_string()),
            }
          }
          CalxSyntax::BrIf(size) => {
            let target_block = blocks_track.peek_block_level(*size)?;

            match target_block {
              BlockData::Loop { from, .. } => ops.push(CalxInstr::BranchIf {
                target: *from,
                base: target_block.branch_base(),
                arity: target_block.branch_arity(),
              }),
              BlockData::Block { to, .. } => ops.push(CalxInstr::BranchIf {
                target: *to,
                base: target_block.branch_base(),
                arity: target_block.branch_arity(),
              }),
              BlockData::If { .. } => return Err("br-if target must be block or loop".to_string()),
            }
            stack_size = stack_size.saturating_sub(1);
          }
          CalxSyntax::BlockEnd(_) => {
            let prev_block = blocks_track.pop_block()?;
            stack_size = prev_block.expected_finish_size();
            ops.push(CalxInstr::Nop)
          }
          CalxSyntax::Call(f_name) => match self.find_func_idx(f_name) {
            Some((idx, f)) => {
              stack_size = stack_size.saturating_sub(f.params_types.len()) + f.ret_types.len();
              ops.push(CalxInstr::Call(idx));
            }
            None => return Err(format!("cannot find function named: {f_name}")),
          },
          CalxSyntax::ReturnCall(f_name) => match self.find_func_idx(f_name) {
            Some((idx, _f)) => {
              stack_size = 0;
              ops.push(CalxInstr::ReturnCall(idx))
            }
            None => return Err(format!("cannot find function named: {f_name}")),
          },
          CalxSyntax::CallImport(f_name) => match &self.imports.get(f_name) {
            Some((_f, size)) => {
              stack_size = stack_size.saturating_sub(*size) + 1;
              ops.push(CalxInstr::CallImport(f_name.to_owned()))
            }
            None => return Err(format!("missing imported function {f_name}")),
          },
          CalxSyntax::Return => {
            stack_size = 0;
            ops.push(CalxInstr::Return);
          }
          CalxSyntax::If { ret_types, else_at, to } => {
            blocks_track.push(BlockData::If {
              ret_types: ret_types.clone(),
              else_to: *else_at,
              to: *to,
              initial_stack_size: stack_size,
            });

            stack_size = stack_size.saturating_sub(1);
            ops.push(CalxInstr::JmpIf(*else_at));
          }
          CalxSyntax::ElseEnd => {
            let prev_block = blocks_track.peek_if()?;
            match prev_block {
              BlockData::If {
                to, initial_stack_size, ..
              } => {
                ops.push(CalxInstr::Jmp(*to));
                stack_size = initial_stack_size.saturating_sub(1);
              }
              _ => return Err("else marker must be inside if".to_string()),
            }
          }
          CalxSyntax::ThenEnd => {
            let prev_block = blocks_track.pop_if()?;
            stack_size = prev_block.expected_finish_size();
            match prev_block {
              BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(to)),
              _ => return Err("then marker must be inside if".to_string()),
            }
          }
          a => {
            let instr: CalxInstr = a.try_into()?;
            let (params_size, ret_size) = instr.stack_arity();
            stack_size = stack_size.saturating_sub(params_size) + ret_size;
            ops.push(instr);
          }
        }
      }

      self.funcs[i].instrs = Rc::new(ops);
    }

    Ok(())
  }

  #[inline(always)]
  fn collect_return_values(&mut self, ret_size: usize) -> Result<(), CalxError> {
    let Some(results_at) = self.stack.len().checked_sub(ret_size) else {
      return Err(self.gen_err(format!(
        "stack size {} does not contain {} return values",
        self.stack.len(),
        ret_size
      )));
    };

    if results_at < self.top_frame.initial_stack_size {
      return Err(self.gen_err(format!(
        "stack size {} does not contain return values above frame base {} for {:?}",
        self.stack.len(),
        self.top_frame.initial_stack_size,
        self.top_frame.ret_types
      )));
    }

    if results_at > self.top_frame.initial_stack_size {
      self.stack.drain(self.top_frame.initial_stack_size..results_at);
    }

    Ok(())
  }

  fn apply_branch_stack(&mut self, base: usize, arity: usize) -> Result<(), CalxError> {
    let absolute_base = self
      .top_frame
      .initial_stack_size
      .checked_add(base)
      .ok_or_else(|| self.gen_err("branch stack base overflow".to_string()))?;
    let Some(values_at) = self.stack.len().checked_sub(arity) else {
      return Err(self.gen_err(format!("branch requires {arity} result value(s)")));
    };
    if values_at < absolute_base {
      return Err(self.gen_err(format!(
        "branch result values overlap target frame base: values at {values_at}, base {absolute_base}"
      )));
    }

    if values_at > absolute_base {
      self.stack.drain(absolute_base..values_at);
    }
    Ok(())
  }

  fn pointer_with_offset(&self, offset: i32) -> Result<usize, CalxError> {
    let pointer = isize::try_from(self.top_frame.pointer)
      .map_err(|_| self.gen_err("instruction pointer does not fit a signed offset".to_string()))?;
    let target = pointer
      .checked_add(offset as isize)
      .ok_or_else(|| self.gen_err(format!("instruction pointer overflow for offset {offset}")))?;
    usize::try_from(target).map_err(|_| self.gen_err(format!("instruction pointer moved before function start by offset {offset}")))
  }

  #[inline(always)]
  fn stack_pop(&mut self) -> Result<Calx, CalxError> {
    let stack_len = self.stack.len();
    if stack_len <= self.top_frame.initial_stack_size {
      Err(self.gen_err(String::from("cannot pop from parent stack")))
    } else {
      match self.stack.pop() {
        Some(value) => Ok(value),
        None => Err(self.gen_err(String::from("cannot pop from empty stack"))),
      }
    }
  }

  #[inline(always)]
  fn stack_pop_right(&mut self) -> Result<(usize, Calx), CalxError> {
    self.check_before_pop_n(2)?;
    let right = match self.stack.pop() {
      Some(value) => value,
      None => return Err(self.gen_err(String::from("cannot pop right operand"))),
    };
    Ok((self.stack.len() - 1, right))
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
      snapshot: Some(Box::new(CalxErrorSnapshot {
        top_frame: self.top_frame.to_owned(),
        stack: self.stack.to_owned(),
        globals: self.globals.to_owned(),
      })),
    }
  }

  fn find_func(&self, name: &str) -> Option<&CalxFunc> {
    self.funcs.iter().find(|x| &*x.name == name)
  }

  fn find_func_idx(&self, name: &str) -> Option<(usize, &CalxFunc)> {
    self.funcs.iter().enumerate().find(|pair| &*pair.1.name == name)
  }
}

/// Runtime or host error with an optional, out-of-line VM snapshot.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxError {
  /// Human-readable diagnostic message; not yet a stable error code.
  pub message: String,
  /// VM state for interpreter-originated traps, absent for raw host errors.
  pub snapshot: Option<Box<CalxErrorSnapshot>>,
}

/// VM state captured only when an execution error originates inside a VM.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxErrorSnapshot {
  /// Operand stack at the failure point.
  pub stack: Vec<Calx>,
  /// Active frame at the failure point.
  pub top_frame: CalxFrame,
  /// Globals at the failure point.
  pub globals: Vec<Calx>,
}

impl fmt::Display for CalxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)?;
    if let Some(snapshot) = &self.snapshot {
      write!(f, "\n{:?}\n{}", snapshot.stack, snapshot.top_frame)?;
    }
    Ok(())
  }
}

impl CalxError {
  /// Creates a host-originated error without inventing VM state.
  pub fn new_raw(s: String) -> Self {
    CalxError {
      message: s,
      snapshot: None,
    }
  }

  /// Returns the captured operand stack when this error originated in a VM.
  pub fn stack(&self) -> Option<&[Calx]> {
    self.snapshot.as_deref().map(|snapshot| snapshot.stack.as_slice())
  }

  /// Returns the captured active frame when this error originated in a VM.
  pub fn top_frame(&self) -> Option<&CalxFrame> {
    self.snapshot.as_deref().map(|snapshot| &snapshot.top_frame)
  }

  /// Returns the captured globals when this error originated in a VM.
  pub fn globals(&self) -> Option<&[Calx]> {
    self.snapshot.as_deref().map(|snapshot| snapshot.globals.as_slice())
  }
}
