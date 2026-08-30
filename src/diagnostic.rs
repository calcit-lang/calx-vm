use core::fmt;
use std::rc::Rc;

use crate::{Calx, ValidationType};

/// Stable identifier for a class of Calx diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCode {
  CirruParse,
  InstructionParse,
  Validation,
  RuntimeTrap,
  HostImport,
}

impl DiagnosticCode {
  /// Stable string intended for tools, tests, and logs.
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::CirruParse => "CALX_PARSE_CIRRU",
      Self::InstructionParse => "CALX_PARSE_INSTRUCTION",
      Self::Validation => "CALX_VALIDATION",
      Self::RuntimeTrap => "CALX_RUNTIME_TRAP",
      Self::HostImport => "CALX_HOST_IMPORT",
    }
  }
}

impl fmt::Display for DiagnosticCode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

/// Pipeline phase that produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticPhase {
  Parse,
  Validation,
  Runtime,
  Host,
}

impl fmt::Display for DiagnosticPhase {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Parse => f.write_str("parse"),
      Self::Validation => f.write_str("validation"),
      Self::Runtime => f.write_str("runtime"),
      Self::Host => f.write_str("host"),
    }
  }
}

/// One source position. Lines and columns are one-based; offsets are byte-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePosition {
  pub line: usize,
  pub column: usize,
  pub offset: usize,
}

impl SourcePosition {
  pub const fn new(line: usize, column: usize, offset: usize) -> Self {
    Self { line, column, offset }
  }
}

/// Half-open source range associated with a parsed Calx expression.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceSpan {
  pub source: Rc<str>,
  pub start: SourcePosition,
  pub end: SourcePosition,
}

impl SourceSpan {
  pub fn new(source: Rc<str>, start: SourcePosition, end: SourcePosition) -> Self {
    Self { source, start, end }
  }

  /// Compact location used by command-line diagnostics.
  pub fn location(&self) -> String {
    format!("{}:{}:{}", self.source, self.start.line, self.start.column)
  }
}

impl fmt::Display for SourceSpan {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}:{}:{}", self.source, self.start.line, self.start.column)
  }
}

/// Borrowed stack data exposed by a diagnostic without allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticStack<'a> {
  ValidationTypes(&'a [ValidationType]),
  RuntimeValues(&'a [Calx]),
}

impl fmt::Display for DiagnosticStack<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::ValidationTypes(values) => write!(f, "{values:?}"),
      Self::RuntimeValues(values) => write!(f, "{values:?}"),
    }
  }
}

/// Common borrowed view over parse, validation, runtime, and host failures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagnosticView<'a> {
  pub code: DiagnosticCode,
  pub phase: DiagnosticPhase,
  pub message: &'a str,
  pub function: Option<&'a str>,
  pub instruction_index: Option<usize>,
  pub span: Option<&'a SourceSpan>,
  pub expected_stack: Option<DiagnosticStack<'a>>,
  pub actual_stack: Option<DiagnosticStack<'a>>,
}

impl fmt::Display for DiagnosticView<'_> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "error[{}] {}", self.code, self.phase)?;
    if let Some(span) = self.span {
      write!(f, " at {span}")?;
    }
    if let Some(function) = self.function {
      write!(f, " in function {function}")?;
    }
    if let Some(index) = self.instruction_index {
      write!(f, " at syntax[{index}]")?;
    }
    write!(f, ": {}", self.message)?;
    if let Some(expected) = self.expected_stack {
      write!(f, "\nexpected stack: {expected}")?;
    }
    if let Some(actual) = self.actual_stack {
      write!(f, "\nactual stack: {actual}")?;
    }
    Ok(())
  }
}
