/*!
 * Calx is a simplied VM from Calcit, but not lower level enough.
 * Data in Calx is mutable, and has basic types and structures, such as Lists.
 * Calx uses a mixed form of prefix and postix instructions.
 *
 * (I'm not equiped enough for building a bytecode VM yet...)
 */

use std::fmt;

/// Simplied from Calcit, but trying to be basic and mutable
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Calx {
  Bool(bool),
  I64(i64),
  F64(f64),
  Str(String),
  List(Vec<Calx>),
}

impl fmt::Display for Calx {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Calx::Bool(b) => f.write_str(&b.to_string()),
      Calx::I64(n) => f.write_str(&n.to_string()),
      Calx::F64(n) => f.write_str(&n.to_string()),
      Calx::Str(s) => f.write_str(&s),
      Calx::List(xs) => {
        f.write_str("(")?;
        let mut at_head = true;
        for x in xs {
          if at_head {
            at_head = false
          } else {
            f.write_str(" ")?;
          }
          x.fmt(f)?;
        }
        f.write_str(")")?;
        Ok(())
      }
    }
  }
}

/// learning from WASM but for dynamic data
pub enum CalxInstr {
  Param, // load variable from parameter
  Local, // new local variable
  LocalSet(usize),
  LocalGet(usize),
  GlobalSet(usize),
  GlobalGet(usize),
  Load(Calx),
  Dup,
  Drop,
  // number operations
  IntAdd,
  IntMul,
  IntRem,
  IntNeg,
  IntShr,
  IntShl,
  Add,
  Mul,
  Div,
  Neg,
  // string operations
  // list operations
  // bool operations
  And,
  Or,
  // control stuctures
  Br(usize),
  BrIf(usize),
  Block(Vec<CalxInstr>),
  Loop(Vec<CalxInstr>),
  Call(usize), // during running, only use index
  Unreachable,
  Nop,
  Quit, // quit and return value
}

pub struct CalxError {
  pub message: String,
  pub instrs: Vec<CalxInstr>,
  pub recent_stack: Vec<CalxInstr>, // maybe partial of stack
}

pub struct CalxFn {
  pub globals: Vec<Calx>, // global variables
  pub locals: Vec<Calx>,  // params + added locals
  pub body: Vec<CalxInstr>,
}

pub struct CalxVM {
  pub stack: Vec<CalxFn>,
  pub funcs: Vec<CalxFn>,
}

impl CalxVM {
  fn new(globals: Vec<Calx>, instrs: Vec<CalxInstr>) {}
  fn eval(idx: usize, params: Vec<Calx>) -> Calx {
    // TODO
    Calx::Bool(true)
  }
}
