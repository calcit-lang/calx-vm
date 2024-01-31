use std::rc::Rc;

use bincode::{Decode, Encode};

use crate::{Calx, CalxType};

/// learning from WASM but for dynamic data
#[derive(Debug, Clone, PartialEq, PartialOrd, Decode, Encode)]
pub enum CalxSyntax {
  /// `local.set`, pop from stack, set value at position
  LocalSet(usize),
  /// `local.tee`, set value at position, and also load to stack
  LocalTee(usize),
  /// `local.get`, get value at position load on stack
  LocalGet(usize),
  /// `local.new`, increase size of array of locals
  LocalNew,
  /// `global.set`, set global value at position
  GlobalSet(usize),
  /// `global.get`, get global value from position
  GlobalGet(usize),
  /// `global.new`, increase size of array of globals
  GlobalNew,
  /// `const`, push value to stack
  Const(Calx),
  /// `dup`, duplicate value on stack
  Dup,
  /// `drop`, drop top value from stack
  Drop,
  /// `i.add`, add two i64 numbers on stack into a i64
  IntAdd,
  /// `i.mul`, multiply two i64 numbers on stack into a i64
  IntMul,
  /// `i.div`, divide two i64 numbers on stack into a i64
  IntDiv,
  /// `i.rem`, remainder of two i64 numbers on stack into a i64
  IntRem,
  /// `i.neg`, negate a i64 number on stack
  IntNeg,
  /// `i.shr`, shift right a i64 number on stack
  IntShr,
  /// `i.shl`, shift left a i64 number on stack
  IntShl,
  /// `i.eq`, equal of two i64 numbers on stack into a bool
  IntEq,
  /// `i.ne`, not equal of two i64 numbers on stack into a bool
  IntNe,
  /// `i.lt`, littler than, compares two i64 numbers on stack
  IntLt,
  /// `i.le`, littler than, or equal, compares two i64 numbers on stack
  IntLe,
  /// `i.gt`, greater than, compares two i64 numbers on stack
  IntGt,
  /// `i.ge`, greater than, or equal, compares two i64 numbers on stack
  IntGe,
  /// `add`, add two f64 numbers on stack into a f64
  Add,
  /// `mul`, multiply two f64 numbers on stack into a f64
  Mul,
  /// `div`, divide two f64 numbers on stack into a f64
  Div,
  /// `neg`, negate a f64 number on stack
  Neg,
  /// TODO list operations
  NewList,
  /// TODO
  ListGet,
  /// TODO
  ListSet,
  /// TODO Link
  NewLink,
  /// TODO
  And,
  /// TODO
  Or,
  /// TODO
  Not,
  /// `block`, creates block, for `block` and `loop`
  Block {
    /// bool to indicate loop
    looped: bool,
    params_types: Rc<Vec<CalxType>>,
    ret_types: Rc<Vec<CalxType>>,
    /// index of `end` instruction
    from: usize,
    /// index of `end` instruction
    to: usize,
  },
  /// `br`, break from block, level `0` indicates the innermost block
  Br(usize),
  /// `br-if`, break from block conditionally, level `0` indicates the innermost block
  BrIf(usize),
  /// (parsed) end of block, for `block` and `loop`
  BlockEnd(bool),
  /// `do`, just a list of instructions nested, used inside `if` area
  Do(Vec<CalxSyntax>),
  /// `echo`, pop and println current value
  Echo,
  /// `call`, call function
  /// TODO optimize with index
  Call(String),
  /// `return-call`, tail recursion call function name
  ReturnCall(String),
  /// `call-import`, call import function
  CallImport(String),
  /// `unreachable`, unreachable panic
  Unreachable,
  /// `nop`, no operation placeholder
  Nop,
  /// `quit`, quit and return value
  Quit(usize),
  /// `return`, return from function
  Return,
  /// `assert`, TODO might also be a foreign function instead
  Assert(String),
  /// `inspect`, inspecting stack
  Inspect,
  /// `if`, takes 1 value from stack, returns values as ret_types
  If {
    ret_types: Rc<Vec<CalxType>>,
    else_at: usize,
    to: usize,
  },
  /// (parsed) end of then instructions
  ThenEnd,
  /// (parsed) end of else instructions
  ElseEnd,
}
