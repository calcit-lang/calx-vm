use std::rc::Rc;

use bincode::{Decode, Encode};

use crate::{Calx, CalxType};

/// learning from WASM but for dynamic data
#[derive(Debug, Clone, PartialEq, PartialOrd, Decode, Encode)]
pub enum CalxSyntax {
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
  Br(usize),
  BrIf(usize),
  Block {
    params_types: Rc<Vec<CalxType>>,
    ret_types: Rc<Vec<CalxType>>,
    /// bool to indicate loop
    looped: bool,
    from: usize,
    to: usize,
  },
  BlockEnd(bool),
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
