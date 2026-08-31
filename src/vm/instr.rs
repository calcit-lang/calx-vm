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
  /// equal of two f64 numbers on stack into a bool
  F64Eq,
  /// not equal of two f64 numbers on stack into a bool
  F64Ne,
  /// less than, compares two f64 numbers on stack
  F64Lt,
  /// less than, or equal, compares two f64 numbers on stack
  F64Le,
  /// greater than, compares two f64 numbers on stack
  F64Gt,
  /// greater than, or equal, compares two f64 numbers on stack
  F64Ge,
  /// return an F64Buffer element count as i64
  F64BufferLen,
  /// checked non-negative integral f64 to i64 index conversion
  F64ToI64Index,
  /// read one F64Buffer element by i64 index
  F64BufferGet,
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
  /// Branch to index, preserving `arity` values above the target frame base
  Branch { target: usize, base: usize, arity: usize },
  /// Jump by offset
  JmpOffset(i32),
  /// Jump to index if top value is true
  JmpIf(usize),
  /// Conditional branch with target-frame stack cleanup
  BranchIf { target: usize, base: usize, arity: usize },
  /// Jump by offset if top value is true
  JmpOffsetIf(i32),
  /// pop and println current value
  Echo,
  /// call function
  Call(usize),
  /// tail recursion
  ReturnCall(usize),
  /// call import
  CallImport(Rc<str>),
  /// call a validated typed import by its stable declaration index
  CallImportIndexed(usize),
  /// unreachable panic
  Unreachable,
  /// no operation placeholder
  Nop,
  /// quit and return error code
  Quit(usize),
  /// return from function
  Return,
  /// TODO might also be a foreign function instead
  Assert(Rc<str>),
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
      CalxSyntax::F64Eq => Ok(Self::F64Eq),
      CalxSyntax::F64Ne => Ok(Self::F64Ne),
      CalxSyntax::F64Lt => Ok(Self::F64Lt),
      CalxSyntax::F64Le => Ok(Self::F64Le),
      CalxSyntax::F64Gt => Ok(Self::F64Gt),
      CalxSyntax::F64Ge => Ok(Self::F64Ge),
      CalxSyntax::F64BufferLen => Ok(Self::F64BufferLen),
      CalxSyntax::F64ToI64Index => Ok(Self::F64ToI64Index),
      CalxSyntax::F64BufferGet => Ok(Self::F64BufferGet),
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
      CalxSyntax::Echo => Ok(Self::Echo),
      CalxSyntax::Unreachable => Ok(Self::Unreachable),
      CalxSyntax::Nop => Ok(Self::Nop),
      CalxSyntax::Quit(a) => Ok(Self::Quit(a.to_owned())),
      CalxSyntax::Return => Ok(Self::Return),
      CalxSyntax::Assert(a) => Ok(Self::Assert(a.to_owned())),
      CalxSyntax::CallImport(a) => Ok(Self::CallImport(a.to_owned())),
      // debug
      CalxSyntax::Inspect => Ok(Self::Inspect),

      // control flow syntax would be compiled
      CalxSyntax::Br(_) => Err("Br should be handled manually".to_string()),
      CalxSyntax::BrIf(_) => Err("BrIf should be handled manually".to_owned()),
      CalxSyntax::Block { .. } => Err("Block should be handled manually".to_string()),
      CalxSyntax::BlockEnd(a) => Err(format!("BlockEnd should be handled manually: {a}")),
      CalxSyntax::Call(_) => Err("Call should be handled manually".to_string()),
      CalxSyntax::ReturnCall(_) => Err("ReturnCall should be handled manually".to_string()),
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
      CalxInstr::F64Eq => (2, 1),
      CalxInstr::F64Ne => (2, 1),
      CalxInstr::F64Lt => (2, 1),
      CalxInstr::F64Le => (2, 1),
      CalxInstr::F64Gt => (2, 1),
      CalxInstr::F64Ge => (2, 1),
      CalxInstr::F64BufferLen => (1, 1),
      CalxInstr::F64ToI64Index => (1, 1),
      CalxInstr::F64BufferGet => (2, 1),
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
      CalxInstr::Branch { .. } => (0, 0),
      CalxInstr::JmpOffset(_) => (0, 0),
      CalxInstr::JmpIf(_) => (1, 0),
      CalxInstr::BranchIf { .. } => (1, 0),
      CalxInstr::JmpOffsetIf(_) => (1, 0),
      CalxInstr::Echo => (1, 0),
      CalxInstr::Call(_) => (0, 0),       // TODO
      CalxInstr::ReturnCall(_) => (0, 0), // TODO
      CalxInstr::CallImport(_) => (0, 0), // import
      CalxInstr::CallImportIndexed(_) => (0, 0),
      CalxInstr::Unreachable => (0, 0), // TODO
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
pub const CALX_INSTR_EDITION: &str = "0.5";
