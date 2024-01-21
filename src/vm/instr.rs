use std::rc::Rc;

use bincode::{Decode, Encode};

use crate::{
  calx::{Calx, CalxType},
  syntax::CalxSyntax,
};

/// learning from WASM but for dynamic data
#[derive(Debug, Clone, PartialEq, PartialOrd, Decode, Encode)]
pub enum CalxInstr {
  // Param, // load variable from parameter
  LocalSet(usize),
  LocalTee(usize), // set and also load to stack
  LocalGet(usize),
  LocalNew,
  GlobalSet(usize),
  GlobalGet(usize),
  GlobalNew,
  Const(Calx),
  Dup,
  Drop,
  // number operations
  IntAdd,
  IntMul,
  IntDiv,
  IntRem,
  IntNeg,
  IntShr,
  IntShl,
  /// equal
  IntEq,
  /// not equal
  IntNe,
  /// littler than
  IntLt,
  /// littler than, or equal
  IntLe,
  /// greater than
  IntGt,
  /// greater than, or equal
  IntGe,
  Add,
  Mul,
  Div,
  Neg,
  // string operations
  // list operations
  NewList,
  ListGet,
  ListSet,
  // Link
  NewLink,
  // bool operations
  And,
  Or,
  Not,
  // control stuctures
  Jmp(usize),       // internal
  JmpOffset(i32),   // internal
  JmpIf(usize),     // internal
  JmpOffsetIf(i32), // internal
  /// pop and println current value
  Echo,
  /// TODO use function name at first, during running, index can be faster
  Call(String),
  /// for tail recursion
  ReturnCall(String),
  CallImport(String),
  Unreachable,
  Nop,
  Quit(usize), // quit and return value
  Return,
  /// TODO might also be a foreign function instead
  Assert(String),
  /// inspecting stack
  Inspect,
  /// if takes 1 value from stack, returns values as ret_types
  If {
    ret_types: Rc<Vec<CalxType>>,
    then_to: usize,
    else_to: usize,
    to: usize,
  },
  EndIf,
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
      CalxSyntax::Call(a) => Ok(Self::Call(a.to_owned())),
      CalxSyntax::ReturnCall(a) => Ok(Self::ReturnCall(a.to_owned())),
      CalxSyntax::CallImport(a) => Ok(Self::CallImport(a.to_owned())),
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
      CalxInstr::If { ret_types, .. } => (1, ret_types.len()),
      CalxInstr::EndIf => (0, 0),
    }
  }
}

/// TODO not sure whether bincode remains compatible after new instruction added
/// use string for some semantics
pub const CALX_INSTR_EDITION: &str = "0.2";
