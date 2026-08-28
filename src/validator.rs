use core::fmt;

use crate::{Calx, CalxFunc, CalxImportsDict, CalxSyntax, CalxType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationType {
  Known(CalxType),
  Dynamic,
}

impl ValidationType {
  fn accepts(self, actual: Self) -> bool {
    self == Self::Dynamic || actual == Self::Dynamic || self == actual
  }
}

impl From<CalxType> for ValidationType {
  fn from(value: CalxType) -> Self {
    Self::Known(value)
  }
}

impl fmt::Display for ValidationType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ValidationType::Known(t) => write!(f, "{t:?}"),
      ValidationType::Dynamic => f.write_str("Dynamic"),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
  pub function: String,
  pub instruction_index: usize,
  pub message: String,
  pub operand_stack: Vec<ValidationType>,
}

impl fmt::Display for ValidationError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "validation error in {} at syntax[{}]: {}\noperand stack: {:?}",
      self.function, self.instruction_index, self.message, self.operand_stack
    )
  }
}

impl std::error::Error for ValidationError {}

/// Kind of a control frame exposed by a validation trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationControlKind {
  Function,
  Block,
  Loop,
  If,
}

/// Observable state of one validation control frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationControlState {
  /// Structure represented by this frame.
  pub kind: ValidationControlKind,
  /// Operand-stack height below the frame's parameters.
  pub height: usize,
  /// Types accepted by a branch to this frame's label.
  pub label_types: Vec<ValidationType>,
  /// Whether the current instruction position is statically unreachable.
  pub unreachable: bool,
}

/// Validation state immediately before and after one expanded syntax instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationStep {
  /// Zero-based index in the function's flattened `CalxSyntax` sequence.
  pub instruction_index: usize,
  /// Instruction validated at this step.
  pub instruction: CalxSyntax,
  /// Typed operand stack before the instruction.
  pub operand_stack_before: Vec<ValidationType>,
  /// Typed operand stack after the instruction.
  pub operand_stack_after: Vec<ValidationType>,
  /// Control stack before the instruction, ordered outermost to innermost.
  pub control_stack_before: Vec<ValidationControlState>,
  /// Control stack after the instruction, ordered outermost to innermost.
  pub control_stack_after: Vec<ValidationControlState>,
}

/// Ordered validation steps for one function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionValidationTrace {
  /// Function name.
  pub function: String,
  /// One step per expanded syntax instruction.
  pub steps: Vec<ValidationStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlKind {
  Function,
  Block,
  Loop,
  If { else_seen: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlFrame {
  kind: ControlKind,
  height: usize,
  start_types: Vec<ValidationType>,
  end_types: Vec<ValidationType>,
  unreachable: bool,
  first_branch_unreachable: Option<bool>,
}

impl ControlFrame {
  fn label_types(&self) -> &[ValidationType] {
    match self.kind {
      ControlKind::Loop => &self.start_types,
      ControlKind::Function | ControlKind::Block | ControlKind::If { .. } => &self.end_types,
    }
  }
}

pub fn validate_program(fns: &[CalxFunc], globals: &[Calx], imports: &CalxImportsDict) -> Result<(), ValidationError> {
  for func in fns {
    Validator::new(func, fns, globals, imports, false).validate()?;
  }
  Ok(())
}

/// Validates a program and records typed operand/control-stack transitions.
///
/// Unlike [`validate_program`], this allocates snapshots intended for teaching
/// tools and should not be used on the interpreter's execution path.
pub fn trace_validation(
  fns: &[CalxFunc],
  globals: &[Calx],
  imports: &CalxImportsDict,
) -> Result<Vec<FunctionValidationTrace>, ValidationError> {
  fns
    .iter()
    .map(|func| {
      Validator::new(func, fns, globals, imports, true)
        .validate()
        .map(|steps| FunctionValidationTrace {
          function: func.name.to_string(),
          steps,
        })
    })
    .collect()
}

struct Validator<'a> {
  func: &'a CalxFunc,
  funcs: &'a [CalxFunc],
  imports: &'a CalxImportsDict,
  operand_stack: Vec<ValidationType>,
  control_stack: Vec<ControlFrame>,
  locals: Vec<ValidationType>,
  globals: Vec<ValidationType>,
  instruction_index: usize,
  record_trace: bool,
}

impl<'a> Validator<'a> {
  fn new(func: &'a CalxFunc, funcs: &'a [CalxFunc], globals: &[Calx], imports: &'a CalxImportsDict, record_trace: bool) -> Self {
    Self {
      func,
      funcs,
      imports,
      operand_stack: vec![],
      control_stack: vec![],
      locals: func.params_types.iter().copied().map(ValidationType::Known).collect(),
      globals: globals.iter().map(|v| ValidationType::Known(v.value_type())).collect(),
      instruction_index: 0,
      record_trace,
    }
  }

  fn validate(mut self) -> Result<Vec<ValidationStep>, ValidationError> {
    let mut steps = vec![];
    let returns = self.known_types(&self.func.ret_types);
    self.push_control(ControlKind::Function, vec![], returns)?;

    for (index, syntax) in self.func.syntax.iter().enumerate() {
      self.instruction_index = index;
      let operand_stack_before = self.record_trace.then(|| self.operand_stack.clone());
      let control_stack_before = self.record_trace.then(|| self.control_state());
      self.validate_instruction(syntax)?;
      if let (Some(operand_stack_before), Some(control_stack_before)) = (operand_stack_before, control_stack_before) {
        steps.push(ValidationStep {
          instruction_index: index,
          instruction: syntax.clone(),
          operand_stack_before,
          operand_stack_after: self.operand_stack.clone(),
          control_stack_before,
          control_stack_after: self.control_state(),
        });
      }
    }

    if self.control_stack.len() != 1 {
      return Err(self.error(format!("{} unclosed control frame(s)", self.control_stack.len() - 1)));
    }
    self.end_control(ControlKind::Function)?;
    Ok(steps)
  }

  fn control_state(&self) -> Vec<ValidationControlState> {
    self
      .control_stack
      .iter()
      .map(|frame| ValidationControlState {
        kind: match frame.kind {
          ControlKind::Function => ValidationControlKind::Function,
          ControlKind::Block => ValidationControlKind::Block,
          ControlKind::Loop => ValidationControlKind::Loop,
          ControlKind::If { .. } => ValidationControlKind::If,
        },
        height: frame.height,
        label_types: frame.label_types().to_vec(),
        unreachable: frame.unreachable,
      })
      .collect()
  }

  fn validate_instruction(&mut self, syntax: &CalxSyntax) -> Result<(), ValidationError> {
    use CalxSyntax::*;

    match syntax {
      LocalSet(index) => {
        let expected = self.local_type(*index)?;
        self.pop_expect(expected)?;
      }
      LocalTee(index) => {
        let expected = self.local_type(*index)?;
        let actual = self.pop_expect(expected)?;
        self.operand_stack.push(actual);
      }
      LocalGet(index) => {
        let ty = self.local_type(*index)?;
        self.operand_stack.push(ty);
      }
      LocalNew => self.locals.push(ValidationType::Dynamic),
      GlobalSet(index) => {
        let expected = self.global_type(*index)?;
        self.pop_expect(expected)?;
      }
      GlobalGet(index) => {
        let ty = self.global_type(*index)?;
        self.operand_stack.push(ty);
      }
      GlobalNew => self.globals.push(ValidationType::Dynamic),
      Const(value) => self.operand_stack.push(ValidationType::Known(value.value_type())),
      Dup => {
        let value = self.pop_any()?;
        self.operand_stack.extend([value, value]);
      }
      Drop | Echo | Assert(_) => {
        self.pop_any()?;
      }
      IntAdd | IntMul | IntDiv | IntRem | IntShr | IntShl => {
        self.pop_expect(CalxType::I64.into())?;
        self.pop_expect(CalxType::I64.into())?;
        self.operand_stack.push(CalxType::I64.into());
      }
      IntNeg => {
        self.pop_expect(CalxType::I64.into())?;
        self.operand_stack.push(CalxType::I64.into());
      }
      IntEq | IntNe | IntLt | IntLe | IntGt | IntGe => {
        self.pop_expect(CalxType::I64.into())?;
        self.pop_expect(CalxType::I64.into())?;
        self.operand_stack.push(CalxType::Bool.into());
      }
      Add | Mul => self.validate_overloaded_numeric()?,
      Div => {
        self.pop_expect(CalxType::F64.into())?;
        self.pop_expect(CalxType::F64.into())?;
        self.operand_stack.push(CalxType::F64.into());
      }
      Neg => {
        self.pop_expect(CalxType::F64.into())?;
        self.operand_stack.push(CalxType::F64.into());
      }
      Block {
        looped,
        params_types,
        ret_types,
        ..
      } => {
        let start_types = self.known_types(params_types);
        let end_types = self.known_types(ret_types);
        self.push_control(if *looped { ControlKind::Loop } else { ControlKind::Block }, start_types, end_types)?;
      }
      Br(depth) => {
        let label_types = self.branch_types(*depth)?;
        self.pop_types(&label_types)?;
        self.mark_unreachable()?;
      }
      BrIf(depth) => {
        self.pop_any()?;
        let label_types = self.branch_types(*depth)?;
        let actual = self.pop_types(&label_types)?;
        self.operand_stack.extend(actual);
      }
      BlockEnd(looped) => {
        self.end_control(if *looped { ControlKind::Loop } else { ControlKind::Block })?;
      }
      If { ret_types, .. } => {
        self.pop_any()?;
        let end_types = self.known_types(ret_types);
        self.push_control(ControlKind::If { else_seen: false }, vec![], end_types)?;
      }
      ElseEnd => self.start_second_if_branch()?,
      ThenEnd => self.end_control(ControlKind::If { else_seen: true })?,
      Call(name) => {
        let callee = self
          .funcs
          .iter()
          .find(|f| f.name.as_ref() == name.as_ref())
          .ok_or_else(|| self.error(format!("unknown function `{name}`")))?;
        let params = self.known_types(&callee.params_types);
        let returns = self.known_types(&callee.ret_types);
        self.pop_types(&params)?;
        self.operand_stack.extend(returns);
      }
      ReturnCall(name) => {
        let callee = self
          .funcs
          .iter()
          .find(|f| f.name.as_ref() == name.as_ref())
          .ok_or_else(|| self.error(format!("unknown function `{name}`")))?;
        let params = self.known_types(&callee.params_types);
        let returns = self.known_types(&callee.ret_types);
        let expected_returns = self.known_types(&self.func.ret_types);
        if returns != expected_returns {
          return Err(self.error(format!(
            "return-call result mismatch: callee returns {returns:?}, current function returns {expected_returns:?}"
          )));
        }
        self.pop_types(&params)?;
        self.mark_unreachable()?;
      }
      CallImport(name) => {
        let (_, arity) = self
          .imports
          .get(name)
          .ok_or_else(|| self.error(format!("unknown import `{name}`")))?;
        for _ in 0..*arity {
          self.pop_any()?;
        }
        self.operand_stack.push(ValidationType::Dynamic);
      }
      Return => {
        let returns = self.known_types(&self.func.ret_types);
        self.pop_types(&returns)?;
        self.mark_unreachable()?;
      }
      Unreachable | Quit(_) => self.mark_unreachable()?,
      Nop | Inspect => {}
      NewList | ListGet | ListSet | NewLink | And | Or | Not => {
        return Err(self.error(format!("instruction is reserved but not implemented: {syntax:?}")));
      }
      Do(_) => return Err(self.error("unexpected `do` node after parsing")),
    }
    Ok(())
  }

  fn validate_overloaded_numeric(&mut self) -> Result<(), ValidationError> {
    let right = self.pop_any()?;
    let left = self.pop_any()?;
    let result = match (left, right) {
      (ValidationType::Known(CalxType::I64), ValidationType::Known(CalxType::I64)) => CalxType::I64.into(),
      (ValidationType::Known(CalxType::F64), ValidationType::Known(CalxType::F64)) => CalxType::F64.into(),
      (ValidationType::Dynamic, _) | (_, ValidationType::Dynamic) => ValidationType::Dynamic,
      _ => return Err(self.error(format!("expected two matching numeric values, got {left} and {right}"))),
    };
    self.operand_stack.push(result);
    Ok(())
  }

  fn local_type(&self, index: usize) -> Result<ValidationType, ValidationError> {
    self
      .locals
      .get(index)
      .copied()
      .ok_or_else(|| self.error(format!("local index {index} is not allocated")))
  }

  fn global_type(&self, index: usize) -> Result<ValidationType, ValidationError> {
    self
      .globals
      .get(index)
      .copied()
      .ok_or_else(|| self.error(format!("global index {index} is not allocated")))
  }

  fn known_types(&self, types: &[CalxType]) -> Vec<ValidationType> {
    types.iter().copied().map(ValidationType::Known).collect()
  }

  fn push_control(
    &mut self,
    kind: ControlKind,
    start_types: Vec<ValidationType>,
    end_types: Vec<ValidationType>,
  ) -> Result<(), ValidationError> {
    self.pop_types(&start_types)?;
    let height = self.operand_stack.len();
    self.operand_stack.extend(start_types.iter().copied());
    self.control_stack.push(ControlFrame {
      kind,
      height,
      start_types,
      end_types,
      unreachable: false,
      first_branch_unreachable: None,
    });
    Ok(())
  }

  fn end_control(&mut self, expected_kind: ControlKind) -> Result<(), ValidationError> {
    let frame = self.current_control()?.clone();
    if !same_control_kind(frame.kind, expected_kind) {
      return Err(self.error(format!(
        "unexpected control end: expected {expected_kind:?}, found {:?}",
        frame.kind
      )));
    }
    self.pop_types(&frame.end_types)?;
    if self.operand_stack.len() != frame.height {
      return Err(self.error(format!(
        "control frame leaves {} extra value(s)",
        self.operand_stack.len().saturating_sub(frame.height)
      )));
    }
    let both_if_branches_unreachable =
      matches!(frame.kind, ControlKind::If { .. }) && frame.first_branch_unreachable == Some(true) && frame.unreachable;
    self.control_stack.pop();
    if both_if_branches_unreachable {
      self.mark_unreachable()?;
    } else {
      self.operand_stack.extend(frame.end_types);
    }
    Ok(())
  }

  fn start_second_if_branch(&mut self) -> Result<(), ValidationError> {
    let frame = self.current_control()?.clone();
    match frame.kind {
      ControlKind::If { else_seen: false } => {}
      ControlKind::If { else_seen: true } => return Err(self.error("duplicate else branch")),
      _ => return Err(self.error("else marker outside if")),
    }

    self.pop_types(&frame.end_types)?;
    if self.operand_stack.len() != frame.height {
      return Err(self.error(format!(
        "if branch leaves {} extra value(s)",
        self.operand_stack.len().saturating_sub(frame.height)
      )));
    }
    self.operand_stack.truncate(frame.height);
    self.operand_stack.extend(frame.start_types.iter().copied());
    let current = self.current_control_mut()?;
    current.kind = ControlKind::If { else_seen: true };
    current.first_branch_unreachable = Some(frame.unreachable);
    current.unreachable = false;
    Ok(())
  }

  fn branch_types(&self, depth: usize) -> Result<Vec<ValidationType>, ValidationError> {
    self
      .control_stack
      .iter()
      .rev()
      .filter(|frame| matches!(frame.kind, ControlKind::Block | ControlKind::Loop))
      .nth(depth)
      .map(|frame| frame.label_types().to_vec())
      .ok_or_else(|| self.error(format!("invalid branch depth {depth}")))
  }

  fn pop_types(&mut self, expected: &[ValidationType]) -> Result<Vec<ValidationType>, ValidationError> {
    let mut actual = Vec::with_capacity(expected.len());
    for expected_type in expected.iter().rev() {
      actual.push(self.pop_expect(*expected_type)?);
    }
    actual.reverse();
    Ok(actual)
  }

  fn pop_expect(&mut self, expected: ValidationType) -> Result<ValidationType, ValidationError> {
    let actual = self.pop_any()?;
    if expected.accepts(actual) {
      Ok(actual)
    } else {
      Err(self.error(format!("expected {expected}, found {actual}")))
    }
  }

  fn pop_any(&mut self) -> Result<ValidationType, ValidationError> {
    let frame = self.current_control()?;
    if self.operand_stack.len() == frame.height && frame.unreachable {
      return Ok(ValidationType::Dynamic);
    }
    if self.operand_stack.len() <= frame.height {
      return Err(self.error("operand stack underflow"));
    }
    self.operand_stack.pop().ok_or_else(|| self.error("operand stack underflow"))
  }

  fn mark_unreachable(&mut self) -> Result<(), ValidationError> {
    let height = self.current_control()?.height;
    self.operand_stack.truncate(height);
    self.current_control_mut()?.unreachable = true;
    Ok(())
  }

  fn current_control(&self) -> Result<&ControlFrame, ValidationError> {
    self.control_stack.last().ok_or_else(|| self.error("missing control frame"))
  }

  fn current_control_mut(&mut self) -> Result<&mut ControlFrame, ValidationError> {
    if self.control_stack.is_empty() {
      return Err(self.error("missing control frame"));
    }
    let index = self.control_stack.len() - 1;
    Ok(&mut self.control_stack[index])
  }

  fn error(&self, message: impl Into<String>) -> ValidationError {
    ValidationError {
      function: self.func.name.to_string(),
      instruction_index: self.instruction_index,
      message: message.into(),
      operand_stack: self.operand_stack.clone(),
    }
  }
}

fn same_control_kind(actual: ControlKind, expected: ControlKind) -> bool {
  matches!(
    (actual, expected),
    (ControlKind::Function, ControlKind::Function)
      | (ControlKind::Block, ControlKind::Block)
      | (ControlKind::Loop, ControlKind::Loop)
      | (ControlKind::If { .. }, ControlKind::If { .. })
  )
}
