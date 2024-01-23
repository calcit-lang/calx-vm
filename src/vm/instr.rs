use std::rc::Rc;

use crate::{calx::Calx, syntax::CalxSyntax};

/// learning from WASM but for dynamic data
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum CalxInstr {
  /// pop from stack, set value at position
  LocalSet(usize),
  /// set and also load to stack
  LocalTee(usize),
  /// get value at position load on stack
  LocalGet(usize),
  /// increase size of array of locals
  LocalNew,
  /// set global value at position
  GlobalSet(usize),
  /// get global value from position
  GlobalGet(usize),
  /// increase size of array of globals
  GlobalNew,
  /// push value to stack
  Const(Calx),
  /// duplicate value on stack
  Dup,
  /// drop top value from stack
  Drop,
  /// add two i64 numbers on stack into a i64
  IntAdd,
  /// multiply two i64 numbers on stack into a i64
  IntMul,
  /// divide two i64 numbers on stack into a i64
  IntDiv,
  /// remainder of two i64 numbers on stack into a i64
  IntRem,
  /// negate a i64 number on stack
  IntNeg,
  /// shift right a i64 number on stack
  IntShr,
  /// shift left a i64 number on stack
  IntShl,
  /// equal of two i64 numbers on stack into a bool
  IntEq,
  /// not equal of two i64 numbers on stack into a bool
  IntNe,
  /// littler than, compares two i64 numbers on stack
  IntLt,
  /// littler than, or equal, compares two i64 numbers on stack
  IntLe,
  /// greater than, compares two i64 numbers on stack
  IntGt,
  /// greater than, or equal, compares two i64 numbers on stack
  IntGe,
  /// add two f64 numbers on stack into a f64
  Add,
  /// multiply two f64 numbers on stack into a f64
  Mul,
  /// divide two f64 numbers on stack into a f64
  Div,
  /// negate a f64 number on stack
  Neg,
  /// TODO
  NewList,
  /// TODO
  ListGet,
  /// TODO
  ListSet,
  /// TODO
  NewLink,
  /// TODO
  And,
  /// TODO
  Or,
  /// TODO
  Not,
  /// Jump to index
  Jmp(usize),
  /// Jump by offset
  JmpOffset(i32),
  /// Jump to index if top value is true
  JmpIf(usize),
  /// Jump by offset if top value is true
  JmpOffsetIf(i32),
  /// pop and println current value
  Echo,
  /// call function
  Call(usize),
  /// tail recursion
  ReturnCall(Rc<str>),
  /// call import
  CallImport(Rc<str>),
  /// unreachable panic
  Unreachable,
  /// no operation placeholder
  Nop,
  /// quit and return error code
  Quit(usize),
  /// return from function
  Return,
  /// TODO might also be a foreign function instead
  Assert(String),
  /// inspecting stack
  Inspect,
}

impl TryFrom<&CalxSyntax> for CalxInstr {
  type Error = String;

  fn try_from(syntax: &CalxSyntax) -> Result<Self, Self::Error> {
    match syntax {
      CalxSyntax::LocalSet(a) => Ok(Self::LocalSet(a.to_owned())),
      CalxSyntax::LocalTee(a) => Ok(Self::LocalTee(a.to_owned())),
      CalxSyntax::LocalGet(a) => Ok(Self::LocalGet(a.to_owned())),
      CalxSyntax::LocalNew => Ok(Self::LocalNew),
      CalxSyntax::GlobalSet(a) => Ok(Self::GlobalSet(a.to_owned())),
      CalxSyntax::GlobalGet(a) => Ok(Self::GlobalGet(a.to_owned())),
      CalxSyntax::GlobalNew => Ok(Self::GlobalNew),
      CalxSyntax::Const(a) => Ok(Self::Const(a.to_owned())),
      CalxSyntax::Dup => Ok(Self::Dup),
      CalxSyntax::Drop => Ok(Self::Drop),
      CalxSyntax::IntAdd => Ok(Self::IntAdd),
      CalxSyntax::IntMul => Ok(Self::IntMul),
      CalxSyntax::IntDiv => Ok(Self::IntDiv),
      CalxSyntax::IntRem => Ok(Self::IntRem),
      CalxSyntax::IntNeg => Ok(Self::IntNeg),
      CalxSyntax::IntShr => Ok(Self::IntShr),
      CalxSyntax::IntShl => Ok(Self::IntShl),
      CalxSyntax::IntEq => Ok(Self::IntEq),
      CalxSyntax::IntNe => Ok(Self::IntNe),
      CalxSyntax::IntLt => Ok(Self::IntLt),
      CalxSyntax::IntLe => Ok(Self::IntLe),
      CalxSyntax::IntGt => Ok(Self::IntGt),
      CalxSyntax::IntGe => Ok(Self::IntGe),
      CalxSyntax::Add => Ok(Self::Add),
      CalxSyntax::Mul => Ok(Self::Mul),
      CalxSyntax::Div => Ok(Self::Div),
      CalxSyntax::Neg => Ok(Self::Neg),
      // string operations
      // list operations
      CalxSyntax::NewList => Ok(Self::NewList),
      CalxSyntax::ListGet => Ok(Self::ListGet),
      CalxSyntax::ListSet => Ok(Self::ListSet),
      // Link
      CalxSyntax::NewLink => Ok(Self::NewLink),
      // bool operations
      CalxSyntax::And => Ok(Self::And),
      CalxSyntax::Or => Ok(Self::Or),
      CalxSyntax::Not => Ok(Self::Not),
      // control stuctures
      CalxSyntax::Br(_) => Err("Br should be handled manually".to_string()),
      CalxSyntax::BrIf(_) => Err("BrIf should be handled manually".to_owned()),
      CalxSyntax::Block { .. } => Err("Block should be handled manually".to_string()),
      CalxSyntax::BlockEnd(a) => Err(format!("BlockEnd should be handled manually: {}", a)),
      CalxSyntax::Echo => Ok(Self::Echo),
      CalxSyntax::Call(_) => Err("Call should be handled manually".to_string()),
      CalxSyntax::ReturnCall(a) => Ok(Self::ReturnCall(Rc::from(a.as_str()))),
      CalxSyntax::CallImport(a) => Ok(Self::CallImport(Rc::from(a.as_str()))),
      CalxSyntax::Unreachable => Ok(Self::Unreachable),
      CalxSyntax::Nop => Ok(Self::Nop),
      CalxSyntax::Quit(a) => Ok(Self::Quit(a.to_owned())),
      CalxSyntax::Return => Ok(Self::Return),
      CalxSyntax::Assert(a) => Ok(Self::Assert(a.to_owned())),
      // debug
      CalxSyntax::Inspect => Ok(Self::Inspect),
      CalxSyntax::If { .. } => Err("If should be handled manually".to_string()),
      CalxSyntax::ThenEnd => Err("ThenEnd should be handled manually".to_string()),
      CalxSyntax::ElseEnd => Err("ElseEnd should be handled manually".to_string()),
      CalxSyntax::Do(_) => Err("do should be handled manually".to_string()),
    }
  }
}

impl CalxInstr {
  /// notice that some of the instrs are special and need to handle manually
  pub fn stack_arity(&self) -> (usize, usize) {
    match self {
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
      CalxInstr::Jmp(_) => (0, 0),
      CalxInstr::JmpOffset(_) => (0, 0),
      CalxInstr::JmpIf(_) => (1, 0),
      CalxInstr::JmpOffsetIf(_) => (1, 0),
      CalxInstr::Echo => (1, 0),
      CalxInstr::Call(_) => (0, 0),       // TODO
      CalxInstr::ReturnCall(_) => (0, 0), // TODO
      CalxInstr::CallImport(_) => (0, 0), // import
      CalxInstr::Unreachable => (0, 0),   // TODO
      CalxInstr::Nop => (0, 0),
      CalxInstr::Quit(_) => (0, 0),
      CalxInstr::Return => (1, 0), // TODO
      CalxInstr::Assert(_) => (1, 0),
      // debug
      CalxInstr::Inspect => (0, 0),
    }
  }
}

/// TODO not sure whether bincode remains compatible after new instruction added
/// use string for some semantics
pub const CALX_INSTR_EDITION: &str = "0.2";
