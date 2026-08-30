//! Calx VM is a toy VM for learning WebAssembly.
//! It is a stack machine, and it is dynamically typed. Being an experiment, for Calcit project.

mod calx;
mod diagnostic;
mod parser;
mod syntax;
mod util;
mod validator;
mod vm;

pub use calx::{Calx, CalxType};
pub use diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticStack, DiagnosticView, SourcePosition, SourceSpan};
pub use parser::{extract_nested, parse_function, parse_program, ParseError, ParsedProgram};
pub use syntax::CalxSyntax;
pub use util::log_calx_value;
pub use validator::{
  trace_validation, validate_program, FunctionValidationTrace, ValidationControlKind, ValidationControlState, ValidationError,
  ValidationStep, ValidationType,
};
pub use vm::{
  frame::CalxFrame, func::CalxFunc, instr::CalxInstr, instr::CALX_INSTR_EDITION, CalxError, CalxErrorSnapshot, CalxImportsDict, CalxVM,
};
