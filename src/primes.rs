/*!
 * Calx is a simplied VM, with dynamic data types, and WASM-inspired control flows.
 * It is a toy project, but trying to speed up calculations for Calcit.
 */

use bincode::{Decode, Encode};
use std::{fmt, rc::Rc};

/// Simplied from Calcit, but trying to be basic and mutable
#[derive(Debug, Clone, PartialEq, PartialOrd, Decode, Encode)]
pub enum Calx {
  Nil,
  Bool(bool),
  I64(i64),
  F64(f64),
  Str(String),
  List(Vec<Calx>),
  // to simultate linked structures
  // Link(Box<Calx>, Box<Calx>, Box<Calx>),
}

impl Calx {
  // for runtime type checking
  pub fn typed_as(&self, t: CalxType) -> bool {
    match self {
      Calx::Nil => t == CalxType::Nil,
      Calx::Bool(_) => t == CalxType::Bool,
      Calx::I64(_) => t == CalxType::I64,
      Calx::F64(_) => t == CalxType::F64,
      Calx::Str(_) => t == CalxType::Str,
      Calx::List(_) => t == CalxType::List,
      // Calx::Link(_, _, _) => t == CalxType::Link,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Decode, Encode)]
pub enum CalxType {
  Nil,
  Bool,
  I64,
  F64,
  Str,
  List,
  Link,
}

impl fmt::Display for Calx {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Calx::Nil => f.write_str("nil"),
      Calx::Bool(b) => f.write_str(&b.to_string()),
      Calx::I64(n) => f.write_str(&n.to_string()),
      Calx::F64(n) => f.write_str(&n.to_string()),
      Calx::Str(s) => f.write_str(s),
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
      } // Calx::Link(..) => f.write_str("TODO LINK"),
    }
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Encode, Decode)]
pub struct CalxFunc {
  pub name: String,
  pub params_types: Vec<CalxType>,
  pub ret_types: Rc<Vec<CalxType>>,
  pub instrs: Rc<Vec<CalxInstr>>,
  pub local_names: Rc<Vec<String>>,
}

impl fmt::Display for CalxFunc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CalxFunc {} (", self.name)?;
    for p in &self.params_types {
      write!(f, "{:?} ", p)?;
    }
    f.write_str("-> ")?;
    for p in &*self.ret_types {
      write!(f, "{:?} ", p)?;
    }
    f.write_str(")")?;
    if !self.local_names.is_empty() {
      f.write_str("\n  local_names:")?;
      for (idx, l) in self.local_names.iter().enumerate() {
        write!(f, " {}_{}", idx, l)?;
      }
      f.write_str(" .")?;
    }
    for (idx, instr) in self.instrs.iter().enumerate() {
      write!(f, "\n  {:02} {:?}", idx, instr)?;
    }
    f.write_str("\n")?;
    Ok(())
  }
}

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
  Br(usize),
  BrIf(usize),
  Jmp(usize),   // internal
  JmpIf(usize), // internal
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
      CalxInstr::Br(_) => (0, 0),
      CalxInstr::BrIf(_) => (1, 0),
      CalxInstr::Jmp(_) => (0, 0),
      CalxInstr::JmpIf(_) => (1, 0),
      CalxInstr::Block { .. } => (0, 0),
      CalxInstr::BlockEnd(_) => (0, 0),
      CalxInstr::Echo => (1, 0),
      CalxInstr::Call(_) => (0, 0),       // TODO
      CalxInstr::ReturnCall(_) => (0, 0), // TODO
      CalxInstr::CallImport(_) => (0, 0), // import
      CalxInstr::Unreachable => (0, 0),   // TODO
      CalxInstr::Nop => (0, 0),
      CalxInstr::Quit(_) => (0, 0),
      CalxInstr::Return => (0, 0), // TODO
      CalxInstr::Assert(_) => (1, 0),
    }
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxError {
  pub message: String,
  pub stack: Vec<Calx>,
  pub top_frame: CalxFrame,
  pub blocks: Vec<BlockData>,
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
      blocks: vec![],
      globals: vec![],
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct BlockData {
  pub looped: bool,
  pub params_types: Rc<Vec<CalxType>>,
  pub ret_types: Rc<Vec<CalxType>>,
  pub from: usize,
  pub to: usize,
  pub initial_stack_size: usize,
}

impl BlockData {
  // size of stack after block finished or breaked
  pub fn expected_finish_size(&self) -> usize {
    self.initial_stack_size - self.params_types.len() + self.ret_types.len()
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxFrame {
  pub locals: Vec<Calx>, // params + added locals
  /** store return values */
  pub instrs: Rc<Vec<CalxInstr>>,
  pub pointer: usize,
  pub initial_stack_size: usize,
  pub blocks_track: Vec<BlockData>,
  pub ret_types: Rc<Vec<CalxType>>,
}

impl Default for CalxFrame {
  fn default() -> Self {
    CalxFrame {
      locals: vec![],
      instrs: Rc::new(vec![]),
      pointer: 0,
      initial_stack_size: 0,
      blocks_track: vec![],
      ret_types: Rc::new(vec![]),
    }
  }
}

impl fmt::Display for CalxFrame {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("CalxFrame ")?;
    write!(f, "_{} (", self.initial_stack_size)?;
    for p in &*self.ret_types {
      write!(f, "{:?} ", p)?;
    }
    write!(f, ") @{}", self.pointer)?;
    for (idx, instr) in self.instrs.iter().enumerate() {
      write!(f, "\n  {:02} {:?}", idx, instr)?;
    }
    f.write_str("\n")?;
    Ok(())
  }
}

/// binary format for saving calx program
/// TODO this is not a valid file format that requires magic code
#[derive(Debug, Clone, PartialEq, PartialOrd, Encode, Decode)]
pub struct CalxBinaryProgram {
  /// updates as instructions update
  pub edition: String,
  pub fns: Vec<CalxFunc>,
}

/// TODO not sure whether bincode remains compatible after new instruction added
/// use string for some semantics
pub const CALX_BINARY_EDITION: &str = "0.1";
