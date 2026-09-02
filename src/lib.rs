//! Calx VM is a toy VM for learning WebAssembly.
//! It is a stack machine for a strict typed Calcit subset, with an explicit
//! legacy profile for the original dynamic embedding API.

mod builder;
mod calx;
mod diagnostic;
mod parser;
mod program;
mod syntax;
mod util;
mod validator;
mod vm;

pub use builder::{BodyBuilder, CalxBuildError, CalxBuildErrorKind, FunctionBuilder, GlobalId, ImportId, LocalId, ProgramBuilder};
pub use calx::{Calx, CalxType};
pub use diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticStack, DiagnosticView, SourcePosition, SourceSpan};
pub use parser::{extract_nested, parse_function, parse_program, ParseError, ParsedProgram};
pub use program::{
  CalxBoundaryType, CalxGlobalDecl, CalxHostBinding, CalxHostBindings, CalxHostCallback, CalxImportDecl, CalxLocalDecl, CalxMutability,
  CalxProgram, CalxProgramError, CalxRunResult, ValidatedProgram,
};
pub use syntax::CalxSyntax;
pub use util::log_calx_value;
pub use validator::{
  trace_typed_validation, trace_validation, validate_program, validate_typed_program, FunctionValidationTrace, ValidationControlKind,
  ValidationControlState, ValidationError, ValidationStep, ValidationType,
};
pub use vm::{
  frame::{CalxFrame, CalxSlot},
  func::CalxFunc,
  instr::CalxInstr,
  instr::CALX_INSTR_EDITION,
  CalxError, CalxErrorSnapshot, CalxImportsDict, CalxTraceError, CalxVM, VmEvent, VmEventKind, VmObserver, VmSlotChange,
  DEFAULT_TRACE_STEP_LIMIT,
};
