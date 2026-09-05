mod block_data;
pub mod frame;
pub mod func;
pub mod instr;

use std::collections::hash_map::HashMap;
use std::rc::Rc;
use std::{fmt, mem, vec};

use crate::calx::Calx;
use crate::diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticStack, DiagnosticView, SourceSpan};
use crate::program::{
  CalxHostBinding, CalxHostBindings, CalxHostCallback, CalxImportDecl, CalxProgram, CalxProgramError, CalxRunResult, ValidatedProgram,
};
use crate::syntax::CalxSyntax;
use crate::vm::block_data::BlockStack;

use self::block_data::BlockData;
use self::frame::{CalxFrame, CalxSlot};
use self::func::CalxFunc;
use self::instr::CalxInstr;

/// Default maximum number of executed VM steps emitted by the trace API and CLI.
pub const DEFAULT_TRACE_STEP_LIMIT: usize = 10_000;

/// Receives owned snapshots from an explicitly traced VM execution.
///
/// Ordinary [`CalxVM::run`] and [`CalxVM::run_typed`] never construct these
/// snapshots. Consumers that need tracing opt in through
/// [`CalxVM::run_traced`].
pub trait VmObserver {
  fn on_event(&mut self, event: VmEvent);
}

/// The execution transition represented by one [`VmEvent`].
#[derive(Debug, Clone, PartialEq)]
pub enum VmEventKind {
  Instruction,
  Call { callee: Rc<str>, tail: bool },
  Return,
  Branch { target: usize, taken: bool },
  LocalWrite { index: usize },
  GlobalWrite { index: usize },
  Trap { message: String },
}

/// One local or global slot changed by a traced instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct VmSlotChange {
  pub index: usize,
  pub before: Option<CalxSlot>,
  pub after: Option<CalxSlot>,
}

/// An owned, deterministic snapshot of one real VM transition.
#[derive(Debug, Clone, PartialEq)]
pub struct VmEvent {
  pub step: usize,
  pub kind: VmEventKind,
  pub function: Rc<str>,
  pub instruction_index: usize,
  pub instruction: Option<CalxInstr>,
  pub source_span: Option<SourceSpan>,
  pub frame_depth_before: usize,
  pub frame_depth_after: usize,
  pub stack_before: Vec<Calx>,
  pub stack_after: Vec<Calx>,
  pub local: Option<VmSlotChange>,
  pub global: Option<VmSlotChange>,
}

/// Explicit reason why a traced execution stopped.
#[derive(Debug, Clone, PartialEq)]
pub enum CalxTraceError {
  Runtime(CalxError),
  LimitExceeded {
    limit: usize,
    function: Rc<str>,
    instruction_index: usize,
    source_span: Option<SourceSpan>,
  },
}

impl fmt::Display for CalxTraceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Runtime(error) => error.fmt(f),
      Self::LimitExceeded {
        limit,
        function,
        instruction_index,
        source_span,
      } => {
        write!(f, "error[CALX_TRACE_LIMIT] runtime")?;
        if let Some(span) = source_span {
          write!(f, " at {span}")?;
        }
        write!(
          f,
          " in function {function} at instruction[{instruction_index}]: trace step limit {limit} exhausted"
        )
      }
    }
  }
}

impl std::error::Error for CalxTraceError {}

#[derive(Debug, Clone)]
struct VmTraceContext {
  function: Rc<str>,
  instruction_index: usize,
  instruction: Option<CalxInstr>,
  source_span: Option<SourceSpan>,
  frame_depth: usize,
  stack: Vec<Calx>,
  local_before: Option<VmSlotChange>,
  global_before: Option<VmSlotChange>,
}

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
  stack: Vec<Calx>,
  globals: Vec<CalxSlot>,
  funcs: Vec<CalxFunc>,
  frames: Vec<CalxFrame>,
  top_frame: CalxFrame,
  imports: CalxImportsDict,
  typed_imports: Vec<CalxHostBinding>,
  strict: bool,
  result: Option<CalxRunResult>,
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
      globals: globals.into_iter().map(CalxSlot::Value).collect(),
      funcs: fns,
      frames: vec![],
      top_frame: main_frame,
      imports,
      typed_imports: vec![],
      strict: false,
      result: None,
    }
  }

  /// Validate, lower, and instantiate a strict typed program.
  pub fn from_program(program: CalxProgram, bindings: CalxHostBindings) -> Result<Self, CalxProgramError> {
    let program = ValidatedProgram::try_from_program(program)?;
    Self::from_validated_program(program, bindings)
  }

  /// Instantiate a program that has already passed strict validation.
  pub fn from_validated_program(program: ValidatedProgram, mut bindings: CalxHostBindings) -> Result<Self, CalxProgramError> {
    let (functions, globals, imports) = program.into_parts();
    let mut typed_imports = Vec::with_capacity(imports.len());
    for declaration in &imports {
      let binding = bindings.remove(&declaration.name).ok_or_else(|| {
        CalxProgramError::new(
          format!("missing host binding for import `{}`", declaration.name),
          None,
          declaration.span.clone(),
        )
      })?;
      validate_host_binding(declaration, &binding)?;
      typed_imports.push(binding);
    }
    if let Some(name) = bindings.keys().min() {
      return Err(CalxProgramError::new(
        format!("host binding `{name}` has no matching import declaration"),
        None,
        None,
      ));
    }

    let main_frame = match functions.iter().find(|function| function.name.as_ref() == "main") {
      Some(main) => CalxFrame {
        name: main.name.clone(),
        initial_stack_size: 0,
        instrs: main.instrs.clone(),
        pointer: 0,
        locals: vec![],
        ret_types: main.ret_types.clone(),
      },
      None => {
        return Err(CalxProgramError::new("main function is required for strict execution", None, None));
      }
    };

    Ok(Self {
      stack: vec![],
      globals: globals.into_iter().map(|global| CalxSlot::Value(global.initial)).collect(),
      funcs: functions,
      frames: vec![],
      top_frame: main_frame,
      imports: HashMap::new(),
      typed_imports,
      strict: true,
      result: None,
    })
  }

  pub fn setup_top_frame(&mut self) -> Result<(), String> {
    self.top_frame.instrs = match self.find_func("main") {
      Some(f) => f.instrs.to_owned(),
      None => return Err("main function is required".to_string()),
    };

    Ok(())
  }

  pub fn functions(&self) -> &[CalxFunc] {
    &self.funcs
  }

  fn make_return(&mut self, result: CalxRunResult) {
    self.result = Some(result);
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
    if self.strict {
      return Err(CalxError::new_raw(
        "strict VM requires run_typed(); legacy run() cannot erase a void result".to_string(),
      ));
    }
    match self.run_inner(args)? {
      CalxRunResult::Void => Ok(Calx::Nil),
      CalxRunResult::Value(value) => Ok(value),
    }
  }

  pub fn run_typed(&mut self, args: Vec<Calx>) -> Result<CalxRunResult, CalxError> {
    if !self.strict {
      return Err(CalxError::new_raw(
        "legacy VM cannot use run_typed(); construct it with CalxVM::from_program".to_string(),
      ));
    }
    let main = self
      .funcs
      .iter()
      .find(|function| function.name.as_ref() == "main")
      .ok_or_else(|| CalxError::new_raw("main function is required".to_string()))?;
    validate_runtime_args(main, &args)?;
    self.run_inner(args)
  }

  /// Execute with an observer over real VM transitions and a hard event limit.
  ///
  /// Strict VMs validate their entry arguments exactly as [`Self::run_typed`]
  /// does. Legacy VMs retain their historic dynamic argument behavior, while
  /// still return an explicit void/value result for tracing.
  pub fn run_traced(&mut self, args: Vec<Calx>, limit: usize, observer: &mut dyn VmObserver) -> Result<CalxRunResult, CalxTraceError> {
    if self.strict {
      let main = self
        .funcs
        .iter()
        .find(|function| function.name.as_ref() == "main")
        .ok_or_else(|| CalxTraceError::Runtime(CalxError::new_raw("main function is required".to_string())))?;
      validate_runtime_args(main, &args).map_err(CalxTraceError::Runtime)?;
    }
    self.run_inner_observed(args, Some(observer), limit)
  }

  fn run_inner(&mut self, args: Vec<Calx>) -> Result<CalxRunResult, CalxError> {
    match self.run_inner_observed(args, None, 0) {
      Ok(result) => Ok(result),
      Err(CalxTraceError::Runtime(error)) => Err(error),
      Err(CalxTraceError::LimitExceeded { .. }) => unreachable!("unobserved execution has no trace limit"),
    }
  }

  fn run_inner_observed(
    &mut self,
    args: Vec<Calx>,
    mut observer: Option<&mut dyn VmObserver>,
    limit: usize,
  ) -> Result<CalxRunResult, CalxTraceError> {
    self.reset_entry_state(args)?;
    self.stack.clear();
    let mut step_count = 0;
    loop {
      // println!("Stack {:?}", self.stack);
      // println!("-- op {} {:?}", self.stack.len(), instr);

      if let Some(result) = self.result.take() {
        return Ok(result);
      }

      let trace = observer.as_ref().map(|_| self.trace_context());
      if let Some(context) = &trace {
        if step_count >= limit {
          return Err(CalxTraceError::LimitExceeded {
            limit,
            function: context.function.clone(),
            instruction_index: context.instruction_index,
            source_span: context.source_span.clone(),
          });
        }
      }

      let quick_continue = match self.step() {
        Ok(value) => value,
        Err(error) => {
          if let Some(context) = trace {
            if let Some(observer) = observer.as_deref_mut() {
              observer.on_event(self.trace_event(
                step_count,
                context,
                VmEventKind::Trap {
                  message: error.message.clone(),
                },
              ));
            }
          }
          return Err(CalxTraceError::Runtime(error));
        }
      };
      if let Some(context) = trace {
        if let Some(observer) = observer.as_deref_mut() {
          let kind = self.trace_event_kind(&context, quick_continue);
          observer.on_event(self.trace_event(step_count, context, kind));
        }
      }
      step_count += 1;
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
          let result = self.take_entry_result(self.top_frame.ret_types.len())?;
          self.make_return(result);
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
        if self.condition_truthy(&v)? {
          self.top_frame.pointer = line;
          return Ok(true); // point reset, goto next loop
        }
      }
      BranchIf { target, base, arity } => {
        let (target, base, arity) = (*target, *base, *arity);
        let condition = self.stack_pop()?;
        if self.condition_truthy(&condition)? {
          self.apply_branch_stack(base, arity)?;
          self.top_frame.pointer = target;
          return Ok(true);
        }
      }
      JmpOffsetIf(l) => {
        let offset = *l;
        let v = self.stack_pop()?;
        if self.condition_truthy(&v)? {
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
          self.top_frame.locals[idx].set(v)
        }
      }
      LocalTee(idx) => {
        let idx = *idx;
        let v = self.stack_pop()?;
        if idx >= self.top_frame.locals.len() {
          return Err(self.gen_err(format!("out of bound in local.tee {idx}")));
        } else {
          self.top_frame.locals[idx].set(v.clone())
        }
        self.stack_push(v);
      }
      LocalGet(idx) => {
        if *idx < self.top_frame.locals.len() {
          let Some(local_val) = self.top_frame.locals[*idx].as_value() else {
            return Err(self.gen_err(format!("trap: read before set for local index {idx}")));
          };
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
          let result = self.take_entry_result(ret_size)?;
          self.make_return(result);
          return Ok(false);
        } else {
          let Some(previous_frame) = self.frames.pop() else {
            return Err(self.gen_err("missing caller frame while returning".to_string()));
          };
          self.top_frame = previous_frame;
        }
      }
      LocalNew => self.top_frame.locals.push(CalxSlot::Uninitialized),
      GlobalSet(idx) => {
        let idx = *idx;
        let v = self.stack_pop()?;
        if idx >= self.globals.len() {
          return Err(self.gen_err(format!("out of bound in global.set {idx}")));
        } else {
          self.globals[idx].set(v)
        }
      }
      GlobalGet(idx) => {
        if *idx < self.globals.len() {
          let Some(value) = self.globals[*idx].as_value().cloned() else {
            return Err(self.gen_err(format!("trap: read before set for global index {idx}")));
          };
          self.stack_push(value)
        } else {
          return Err(self.gen_err(format!("invalid index for global.get {idx}")));
        }
      }
      GlobalNew => self.globals.push(CalxSlot::Uninitialized),
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
      F64Eq => self.compare_f64("eq", |left, right| left == right)?,
      F64Ne => self.compare_f64("ne", |left, right| left != right)?,
      F64Lt => self.compare_f64("lt", |left, right| left < right)?,
      F64Le => self.compare_f64("le", |left, right| left <= right)?,
      F64Gt => self.compare_f64("gt", |left, right| left > right)?,
      F64Ge => self.compare_f64("ge", |left, right| left >= right)?,
      F64BufferLen => {
        self.check_before_pop()?;
        let top = self.stack.len() - 1;
        let length = match &self.stack[top] {
          Calx::F64Buffer(values) => values.len(),
          value => return Err(self.gen_err(format!("f64-buffer.len expected F64Buffer, found {:?}", value.value_type()))),
        };
        let length =
          i64::try_from(length).map_err(|_| self.gen_err(format!("trap: f64-buffer.len length {length} does not fit i64")))?;
        self.stack[top] = Calx::I64(length);
      }
      F64ToI64Index => {
        self.check_before_pop()?;
        let top = self.stack.len() - 1;
        let value = match self.stack[top] {
          Calx::F64(value) => value,
          ref value => return Err(self.gen_err(format!("f64.to-i64-index expected F64, found {:?}", value.value_type()))),
        };
        const I64_INDEX_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
        if !value.is_finite() || value.fract() != 0.0 || !(0.0..I64_INDEX_UPPER_EXCLUSIVE).contains(&value) {
          return Err(self.gen_err(format!(
            "trap: f64.to-i64-index invalid value {value:?}; expected finite integral 0 <= value < 2^63"
          )));
        }
        self.stack[top] = Calx::I64(value as i64);
      }
      F64BufferGet => {
        let (buffer_at, index) = self.stack_pop_right()?;
        let index = match index {
          Calx::I64(index) => index,
          value => return Err(self.gen_err(format!("f64-buffer.get expected I64 index, found {:?}", value.value_type()))),
        };
        let values = match &self.stack[buffer_at] {
          Calx::F64Buffer(values) => values,
          value => return Err(self.gen_err(format!("f64-buffer.get expected F64Buffer, found {:?}", value.value_type()))),
        };
        let length = values.len();
        let element_at = usize::try_from(index)
          .map_err(|_| self.gen_err(format!("trap: f64-buffer.get index {index} is out of bounds for length {length}")))?;
        let value = values
          .get(element_at)
          .copied()
          .ok_or_else(|| self.gen_err(format!("trap: f64-buffer.get index {index} is out of bounds for length {length}")))?;
        self.stack[buffer_at] = Calx::F64(value);
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
        let mut locals = self.stack.split_off(next_size).into_iter().map(CalxSlot::Value).collect::<Vec<_>>();
        if self.strict {
          locals.extend(std::iter::repeat_n(CalxSlot::Uninitialized, f.locals.len()));
        }

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
        let Some(f) = self.funcs.get(*idx) else {
          return Err(self.gen_err(format!("invalid function index for return-call: {idx}")));
        };

        let instrs = &f.instrs;
        let ret_types = f.ret_types.clone();
        let f_name = f.name.clone();

        let n = f.params_types.len();
        self.check_before_pop_n(n)?;

        let args_at = self.stack.len() - n;
        // A tail call replaces values, not the locals allocation. Move the
        // arguments out of the operand tail while retaining the caller's prefix.
        // Clearing first releases old buffers and prevents stale initialized slots.
        // Capacity follows the widest layout in this tail-call chain; ordinary
        // returns and entry resets keep their existing frame-release behavior.
        self.top_frame.locals.clear();
        self.top_frame.locals.extend(self.stack.drain(args_at..).map(CalxSlot::Value));
        if self.strict {
          self
            .top_frame
            .locals
            .extend(std::iter::repeat_n(CalxSlot::Uninitialized, f.locals.len()));
        }
        self.stack.truncate(self.top_frame.initial_stack_size);
        self.top_frame.name = f_name;
        self.top_frame.pointer = 0;
        self.top_frame.instrs = instrs.to_owned();
        self.top_frame.ret_types = ret_types;

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
      CallImportIndexed(index) => {
        let Some(binding) = self.typed_imports.get(*index).cloned() else {
          return Err(self.gen_err(format!("invalid typed import index {index}")));
        };
        let arity = binding.params().len();
        self.check_before_pop_n(arity)?;
        let args = self.stack.split_off(self.stack.len() - arity);
        for (position, (value, expected)) in args.iter().zip(binding.params()).enumerate() {
          if value.value_type() != *expected {
            return Err(CalxError::new_raw(format!(
              "typed host import argument {position} expected {expected:?}, found {:?}",
              value.value_type()
            )));
          }
        }
        match binding.callback() {
          CalxHostCallback::Void(callback) => callback(&args).map_err(host_callback_error)?,
          CalxHostCallback::Value(callback) => {
            let value = callback(&args).map_err(host_callback_error)?;
            let Some(expected) = binding.result() else {
              return Err(CalxError::new_raw(
                "value host callback is missing a declared result type".to_string(),
              ));
            };
            if value.value_type() != expected {
              return Err(CalxError::new_raw(format!(
                "typed host import result expected {expected:?}, found {:?}",
                value.value_type()
              )));
            }
            self.stack_push(value);
          }
        }
      }
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
        if self.condition_truthy(&v)? {
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
    let globals = self
      .globals
      .iter()
      .map(|slot| match slot {
        CalxSlot::Value(value) => Ok(value.clone()),
        CalxSlot::Uninitialized => Err("legacy globals must be initialized before preprocessing".to_string()),
      })
      .collect::<Result<Vec<_>, _>>()?;
    crate::validate_program(&self.funcs, &globals, &self.imports).map_err(|e| e.to_string())?;
    lower_functions(&mut self.funcs, LoweringImports::Legacy(&self.imports), verbose)
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

  fn take_entry_result(&mut self, ret_size: usize) -> Result<CalxRunResult, CalxError> {
    match ret_size {
      0 => Ok(CalxRunResult::Void),
      1 => self
        .stack
        .pop()
        .map(CalxRunResult::Value)
        .ok_or_else(|| self.gen_err("return without value".to_string())),
      count => Err(self.gen_err(format!("typed entry result supports zero or one value, found {count}"))),
    }
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
  fn compare_f64(&mut self, operation: &str, compare: impl FnOnce(f64, f64) -> bool) -> Result<(), CalxError> {
    let (left, right) = self.stack_pop_right()?;
    let result = match (&self.stack[left], &right) {
      (Calx::F64(left), Calx::F64(right)) => compare(*left, *right),
      (left, right) => return Err(self.gen_err(format!("expected 2 floats to {operation} compare, {left:?} {right:?}"))),
    };
    self.stack[left] = Calx::Bool(result);
    Ok(())
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

  fn condition_truthy(&self, value: &Calx) -> Result<bool, CalxError> {
    match value {
      Calx::F64Buffer(_) => Err(self.gen_err(String::from("F64Buffer does not participate in truthiness"))),
      _ => Ok(value.truthy()),
    }
  }

  #[inline(always)]
  fn stack_push(&mut self, x: Calx) {
    self.stack.push(x)
  }

  fn reset_entry_state(&mut self, args: Vec<Calx>) -> Result<(), CalxTraceError> {
    let (name, instrs, ret_types, local_count) = self
      .find_func("main")
      .map(|main| (main.name.clone(), main.instrs.clone(), main.ret_types.clone(), main.locals.len()))
      .ok_or_else(|| CalxTraceError::Runtime(CalxError::new_raw("main function is required".to_string())))?;
    let mut locals: Vec<CalxSlot> = args.into_iter().map(CalxSlot::Value).collect();
    if self.strict {
      locals.extend(std::iter::repeat_n(CalxSlot::Uninitialized, local_count));
    }
    self.result = None;
    self.frames.clear();
    self.top_frame = CalxFrame {
      name,
      locals,
      instrs,
      pointer: 0,
      initial_stack_size: 0,
      ret_types,
    };
    Ok(())
  }

  fn trace_context(&self) -> VmTraceContext {
    let instruction_index = self.top_frame.pointer;
    let instruction = self.top_frame.instrs.get(instruction_index).cloned();
    let local_before = trace_local_slot(&instruction, &self.top_frame);
    let global_before = trace_global_slot(&instruction, &self.globals);
    let source_span = self
      .find_func(&self.top_frame.name)
      .and_then(|function| function.source_spans.get(instruction_index))
      .cloned()
      .flatten();
    VmTraceContext {
      function: self.top_frame.name.clone(),
      instruction_index,
      instruction,
      source_span,
      frame_depth: self.frames.len(),
      stack: self.stack.clone(),
      local_before,
      global_before,
    }
  }

  fn trace_event_kind(&self, context: &VmTraceContext, quick_continue: bool) -> VmEventKind {
    match context.instruction.as_ref() {
      Some(CalxInstr::Call(_)) | Some(CalxInstr::ReturnCall(_)) => VmEventKind::Instruction,
      Some(CalxInstr::Jmp(target)) | Some(CalxInstr::Branch { target, .. }) => VmEventKind::Branch {
        target: *target,
        taken: true,
      },
      Some(CalxInstr::JmpIf(target)) | Some(CalxInstr::BranchIf { target, .. }) => VmEventKind::Branch {
        target: *target,
        taken: quick_continue,
      },
      Some(CalxInstr::JmpOffset(offset)) | Some(CalxInstr::JmpOffsetIf(offset)) => VmEventKind::Branch {
        target: trace_offset_target(context.instruction_index, *offset),
        taken: quick_continue,
      },
      Some(CalxInstr::Return) | None => VmEventKind::Return,
      Some(CalxInstr::LocalSet(index)) | Some(CalxInstr::LocalTee(index)) => VmEventKind::LocalWrite { index: *index },
      Some(CalxInstr::LocalNew) => VmEventKind::LocalWrite {
        index: self.top_frame.locals.len().saturating_sub(1),
      },
      Some(CalxInstr::GlobalSet(index)) => VmEventKind::GlobalWrite { index: *index },
      Some(CalxInstr::GlobalNew) => VmEventKind::GlobalWrite {
        index: self.globals.len().saturating_sub(1),
      },
      _ => VmEventKind::Instruction,
    }
  }

  fn trace_event(&self, step: usize, context: VmTraceContext, kind: VmEventKind) -> VmEvent {
    let (local, global) = if matches!(kind, VmEventKind::Trap { .. }) {
      (None, None)
    } else {
      (
        context.local_before.map(|mut change| {
          change.after = self.top_frame.locals.get(change.index).cloned();
          change
        }),
        context.global_before.map(|mut change| {
          change.after = self.globals.get(change.index).cloned();
          change
        }),
      )
    };
    let kind = match (context.instruction.as_ref(), kind) {
      (Some(CalxInstr::Call(index)), VmEventKind::Instruction) => VmEventKind::Call {
        callee: self
          .funcs
          .get(*index)
          .map(|function| function.name.clone())
          .unwrap_or_else(|| Rc::from("<invalid-call>")),
        tail: false,
      },
      (Some(CalxInstr::ReturnCall(index)), VmEventKind::Instruction) => VmEventKind::Call {
        callee: self
          .funcs
          .get(*index)
          .map(|function| function.name.clone())
          .unwrap_or_else(|| Rc::from("<invalid-call>")),
        tail: true,
      },
      (_, kind) => kind,
    };
    VmEvent {
      step,
      kind,
      function: context.function,
      instruction_index: context.instruction_index,
      instruction: context.instruction,
      source_span: context.source_span,
      frame_depth_before: context.frame_depth,
      frame_depth_after: self.frames.len(),
      stack_before: context.stack,
      stack_after: self.stack.clone(),
      local,
      global,
    }
  }

  fn gen_err(&self, s: String) -> CalxError {
    let source_span = self
      .find_func(&self.top_frame.name)
      .and_then(|function| function.source_spans.get(self.top_frame.pointer))
      .cloned()
      .flatten();
    CalxError {
      message: s,
      snapshot: Some(Box::new(CalxErrorSnapshot {
        code: DiagnosticCode::RuntimeTrap,
        source_span,
        top_frame: self.top_frame.to_owned(),
        stack: self.stack.to_owned(),
        globals: self.globals.to_owned(),
      })),
    }
  }

  fn find_func(&self, name: &str) -> Option<&CalxFunc> {
    self.funcs.iter().find(|x| &*x.name == name)
  }
}

fn trace_local_slot(instruction: &Option<CalxInstr>, frame: &CalxFrame) -> Option<VmSlotChange> {
  let index = match instruction {
    Some(CalxInstr::LocalSet(index)) | Some(CalxInstr::LocalTee(index)) => *index,
    Some(CalxInstr::LocalNew) => frame.locals.len(),
    _ => return None,
  };
  Some(VmSlotChange {
    index,
    before: frame.locals.get(index).cloned(),
    after: None,
  })
}

fn trace_offset_target(instruction_index: usize, offset: i32) -> usize {
  instruction_index.checked_add_signed(offset as isize).unwrap_or(instruction_index)
}

fn trace_global_slot(instruction: &Option<CalxInstr>, globals: &[CalxSlot]) -> Option<VmSlotChange> {
  let index = match instruction {
    Some(CalxInstr::GlobalSet(index)) => *index,
    Some(CalxInstr::GlobalNew) => globals.len(),
    _ => return None,
  };
  Some(VmSlotChange {
    index,
    before: globals.get(index).cloned(),
    after: None,
  })
}

enum LoweringImports<'a> {
  Legacy(&'a CalxImportsDict),
  Typed(&'a [CalxImportDecl]),
}

pub(crate) fn lower_typed_functions(functions: &mut [CalxFunc], imports: &[CalxImportDecl]) -> Result<(), String> {
  lower_functions(functions, LoweringImports::Typed(imports), false)
}

fn lower_functions(functions: &mut [CalxFunc], imports: LoweringImports<'_>, verbose: bool) -> Result<(), String> {
  for index in 0..functions.len() {
    let mut stack_size = 0;
    let mut ops = vec![];
    let mut blocks_track = BlockStack::new();
    let function = &functions[index];

    if verbose {
      println!(
        "\nFUNC {}\n  initial stack size: {}\n  ret_size {}",
        function.name,
        stack_size,
        function.ret_types.len()
      );
    }

    for syntax in function.syntax.iter() {
      if verbose {
        println!("{stack_size} * {syntax:?}");
      }
      match syntax {
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
        CalxSyntax::Br(depth) => {
          let target_block = blocks_track.peek_block_level(*depth)?;
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
        CalxSyntax::BrIf(depth) => {
          let target_block = blocks_track.peek_block_level(*depth)?;
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
          let previous = blocks_track.pop_block()?;
          stack_size = previous.expected_finish_size();
          ops.push(CalxInstr::Nop);
        }
        CalxSyntax::Call(name) => {
          let Some((callee_index, callee)) = functions
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name.as_ref() == name.as_ref())
          else {
            return Err(format!("cannot find function named: {name}"));
          };
          stack_size = stack_size.saturating_sub(callee.params_types.len()) + callee.ret_types.len();
          ops.push(CalxInstr::Call(callee_index));
        }
        CalxSyntax::ReturnCall(name) => {
          let Some((callee_index, _)) = functions
            .iter()
            .enumerate()
            .find(|(_, candidate)| candidate.name.as_ref() == name.as_ref())
          else {
            return Err(format!("cannot find function named: {name}"));
          };
          stack_size = 0;
          ops.push(CalxInstr::ReturnCall(callee_index));
        }
        CalxSyntax::CallImport(name) => match &imports {
          LoweringImports::Legacy(entries) => {
            let Some((_, arity)) = entries.get(name) else {
              return Err(format!("missing imported function {name}"));
            };
            stack_size = stack_size.saturating_sub(*arity) + 1;
            ops.push(CalxInstr::CallImport(name.clone()));
          }
          LoweringImports::Typed(entries) => {
            let Some((import_index, declaration)) = entries
              .iter()
              .enumerate()
              .find(|(_, declaration)| declaration.name.as_ref() == name.as_ref())
            else {
              return Err(format!("missing imported function {name}"));
            };
            stack_size = stack_size.saturating_sub(declaration.params.len()) + usize::from(declaration.result.is_some());
            ops.push(CalxInstr::CallImportIndexed(import_index));
          }
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
          let previous = blocks_track.peek_if()?;
          match previous {
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
          let previous = blocks_track.pop_if()?;
          stack_size = previous.expected_finish_size();
          match previous {
            BlockData::If { to, .. } => ops.push(CalxInstr::Jmp(to)),
            _ => return Err("then marker must be inside if".to_string()),
          }
        }
        other => {
          let instr: CalxInstr = other.try_into()?;
          let (params_size, result_size) = instr.stack_arity();
          stack_size = stack_size.saturating_sub(params_size) + result_size;
          ops.push(instr);
        }
      }
    }

    functions[index].instrs = Rc::new(ops);
  }
  Ok(())
}

fn validate_host_binding(declaration: &CalxImportDecl, binding: &CalxHostBinding) -> Result<(), CalxProgramError> {
  let declared_params = declaration
    .params
    .iter()
    .map(|boundary| match boundary {
      crate::CalxBoundaryType::Known(value_type) => Ok(*value_type),
      crate::CalxBoundaryType::Dynamic => Err(CalxProgramError::new(
        format!("strict import `{}` cannot bind a Dynamic parameter", declaration.name),
        None,
        declaration.span.clone(),
      )),
    })
    .collect::<Result<Vec<_>, _>>()?;
  let declared_result = match declaration.result {
    Some(crate::CalxBoundaryType::Known(value_type)) => Some(value_type),
    Some(crate::CalxBoundaryType::Dynamic) => {
      return Err(CalxProgramError::new(
        format!("strict import `{}` cannot bind a Dynamic result", declaration.name),
        None,
        declaration.span.clone(),
      ));
    }
    None => None,
  };

  if binding.params() != declared_params || binding.result() != declared_result {
    return Err(CalxProgramError::new(
      format!(
        "host binding `{}` signature mismatch: guest {:?} -> {:?}, host {:?} -> {:?}",
        declaration.name,
        declared_params,
        declared_result,
        binding.params(),
        binding.result()
      ),
      None,
      declaration.span.clone(),
    ));
  }
  Ok(())
}

fn validate_runtime_args(function: &CalxFunc, args: &[Calx]) -> Result<(), CalxError> {
  if args.len() != function.params_types.len() {
    return Err(CalxError::new_raw(format!(
      "main expected {} argument(s), found {}",
      function.params_types.len(),
      args.len()
    )));
  }
  for (index, (value, expected)) in args.iter().zip(function.params_types.iter()).enumerate() {
    if value.value_type() != *expected {
      return Err(CalxError::new_raw(format!(
        "main argument {index} expected {expected:?}, found {:?}",
        value.value_type()
      )));
    }
  }
  Ok(())
}

fn host_callback_error(error: CalxError) -> CalxError {
  CalxError::new_raw(error.message)
}

/// Runtime or host error with an optional, out-of-line VM snapshot.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxError {
  /// Human-readable diagnostic message; use `code()` for stable matching.
  pub message: String,
  /// VM state for interpreter-originated traps, absent for raw host errors.
  pub snapshot: Option<Box<CalxErrorSnapshot>>,
}

/// VM state captured only when an execution error originates inside a VM.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxErrorSnapshot {
  /// Stable diagnostic code for the interpreter-originated failure.
  pub code: DiagnosticCode,
  /// Source expression at the active instruction, when source-aware parsing was used.
  pub source_span: Option<SourceSpan>,
  /// Operand stack at the failure point.
  pub stack: Vec<Calx>,
  /// Active frame at the failure point.
  pub top_frame: CalxFrame,
  /// Globals at the failure point.
  pub globals: Vec<CalxSlot>,
}

impl fmt::Display for CalxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.diagnostic().fmt(f)?;
    if let Some(snapshot) = &self.snapshot {
      write!(f, "\n{}", snapshot.top_frame)?;
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

  /// Common structured view without cloning the optional VM snapshot.
  pub fn diagnostic(&self) -> DiagnosticView<'_> {
    match self.snapshot.as_deref() {
      Some(snapshot) => DiagnosticView {
        code: snapshot.code,
        phase: DiagnosticPhase::Runtime,
        message: &self.message,
        function: Some(&snapshot.top_frame.name),
        instruction_index: Some(snapshot.top_frame.pointer),
        span: snapshot.source_span.as_ref(),
        expected_stack: None,
        actual_stack: Some(DiagnosticStack::RuntimeValues(&snapshot.stack)),
      },
      None => DiagnosticView {
        code: DiagnosticCode::HostImport,
        phase: DiagnosticPhase::Host,
        message: &self.message,
        function: None,
        instruction_index: None,
        span: None,
        expected_stack: None,
        actual_stack: None,
      },
    }
  }

  /// Stable diagnostic code for this error.
  pub fn code(&self) -> DiagnosticCode {
    self.diagnostic().code
  }

  /// Source expression for interpreter-originated errors.
  pub fn source_span(&self) -> Option<&SourceSpan> {
    self.snapshot.as_deref().and_then(|snapshot| snapshot.source_span.as_ref())
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
  pub fn globals(&self) -> Option<&[CalxSlot]> {
    self.snapshot.as_deref().map(|snapshot| snapshot.globals.as_slice())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{CalxImportDecl, CalxType};

  #[test]
  fn tail_call_capacity_grows_for_wider_layout_then_stays_bounded() {
    let parsed = crate::parse_program(
      "capacity.cirru",
      r#"fn main (-> i64)
  const 1
  const 2
  const 3
  return-call wide

fn wide (i64 i64 i64 -> i64)
  local $a i64
  local $b i64
  local $c i64
  local $d i64
  local $e i64
  const 0
  return-call narrow

fn narrow (i64 -> i64)
  const 1
  const 2
  const 3
  return-call wide"#,
    )
    .unwrap();
    let mut vm = CalxVM::from_program(parsed.into_program().unwrap(), CalxHostBindings::new()).unwrap();
    vm.reset_entry_state(vec![]).unwrap();
    let mut widest_capacity = 0;
    for transition in 0..500 {
      while !matches!(vm.top_frame.instrs[vm.top_frame.pointer], CalxInstr::ReturnCall(_)) {
        if !vm.step().unwrap() {
          vm.top_frame.pointer += 1;
        }
      }
      let previous_pointer = vm.top_frame.locals.as_ptr();
      let previous_capacity = vm.top_frame.locals.capacity();
      assert!(vm.step().unwrap());
      let required = vm.top_frame.locals.len();
      if required <= previous_capacity {
        assert_eq!(vm.top_frame.locals.as_ptr(), previous_pointer);
        assert_eq!(vm.top_frame.locals.capacity(), previous_capacity);
      }
      if transition == 0 {
        widest_capacity = vm.top_frame.locals.capacity();
      }
      assert_eq!(vm.top_frame.locals.capacity(), widest_capacity);
      assert!(vm.frames.is_empty());
      assert!(vm.stack.is_empty());
      if vm.top_frame.name.as_ref() == "wide" {
        assert_eq!(required, 8);
        assert!(vm.top_frame.locals[3..].iter().all(|slot| *slot == CalxSlot::Uninitialized));
      } else {
        assert_eq!(required, 1);
      }
    }
  }

  #[test]
  fn tail_call_operand_underflow_does_not_clear_the_current_frame() {
    let parsed = crate::parse_program(
      "underflow.cirru",
      "fn main (-> i64)\n  const 1\n  return-call identity\nfn identity (i64 -> i64)\n  local.get 0\n  return",
    )
    .unwrap();
    let mut vm = CalxVM::from_program(parsed.into_program().unwrap(), CalxHostBindings::new()).unwrap();
    vm.reset_entry_state(vec![]).unwrap();
    // Bypass validated execution to exercise the runtime guard before any mutation.
    vm.top_frame.pointer = 1;
    let before = vm.top_frame.clone();
    let error = vm.step().unwrap_err();
    assert_eq!(vm.top_frame, before);
    assert!(vm.stack.is_empty());
    assert_eq!(error.diagnostic().function, Some("main"));
    assert_eq!(error.diagnostic().span.unwrap().start.line, 3);
  }

  fn first_buffer_value(values: &[Calx]) -> Result<Calx, CalxError> {
    let [Calx::F64Buffer(buffer)] = values else {
      return Err(CalxError::new_raw(format!("expected one F64Buffer, got {values:?}")));
    };
    Ok(Calx::F64(buffer.first().copied().unwrap_or(0.0)))
  }

  #[test]
  fn typed_import_runtime_rechecks_actual_buffer_argument_variant() {
    let main = CalxFunc::new(
      "main",
      vec![CalxType::F64Buffer],
      vec![CalxType::F64],
      vec![
        CalxSyntax::LocalGet(0),
        CalxSyntax::CallImport(Rc::from("first-value")),
        CalxSyntax::Return,
      ],
    );
    let import = CalxImportDecl::new("first-value", vec![CalxType::F64Buffer], Some(CalxType::F64));
    let program = CalxProgram::try_new(vec![main], vec![], vec![import]).expect("valid typed buffer program");
    let mut bindings = CalxHostBindings::new();
    bindings.insert(
      Rc::from("first-value"),
      CalxHostBinding::value(vec![CalxType::F64Buffer], CalxType::F64, first_buffer_value).expect("valid binding"),
    );
    let mut vm = CalxVM::from_program(program, bindings).expect("instantiate strict VM");

    // Bypass the public entry check to exercise the host boundary's
    // defense-in-depth check against malformed runtime state.
    let error = vm
      .run_inner(vec![Calx::List(vec![Calx::F64(1.0)])])
      .expect_err("actual List argument must not cross an F64Buffer import boundary");
    assert!(error.message.contains("argument 0 expected F64Buffer, found List"), "{error}");
  }
}
