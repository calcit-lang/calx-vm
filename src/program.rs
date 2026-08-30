use core::fmt;
use std::collections::HashSet;
use std::rc::Rc;

use crate::diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticView, SourceSpan};
use crate::{Calx, CalxError, CalxFunc, CalxSyntax, CalxType, ValidationError};

/// A boundary contract retained by parsed and legacy metadata.
///
/// Strict programs only accept `Known` values whose type is neither `Nil` nor
/// the not-yet-executable `Link` placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum CalxBoundaryType {
  Known(CalxType),
  Dynamic,
}

impl fmt::Display for CalxBoundaryType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Known(value_type) => write!(f, "{value_type:?}"),
      Self::Dynamic => f.write_str("Dynamic"),
    }
  }
}

impl From<CalxType> for CalxBoundaryType {
  fn from(value: CalxType) -> Self {
    Self::Known(value)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct CalxLocalDecl {
  pub name: Rc<str>,
  pub value_type: CalxBoundaryType,
  pub span: Option<SourceSpan>,
}

impl CalxLocalDecl {
  pub fn new(name: impl Into<Rc<str>>, value_type: CalxType) -> Self {
    Self {
      name: name.into(),
      value_type: value_type.into(),
      span: None,
    }
  }

  pub fn with_span(mut self, span: Option<SourceSpan>) -> Self {
    self.span = span;
    self
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalxMutability {
  Const,
  Mut,
}

impl fmt::Display for CalxMutability {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Const => f.write_str("const"),
      Self::Mut => f.write_str("mut"),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalxGlobalDecl {
  pub name: Rc<str>,
  pub value_type: CalxBoundaryType,
  pub mutability: CalxMutability,
  pub initial: Calx,
  pub span: Option<SourceSpan>,
}

impl CalxGlobalDecl {
  pub fn new(name: impl Into<Rc<str>>, value_type: CalxType, mutability: CalxMutability, initial: Calx) -> Self {
    Self {
      name: name.into(),
      value_type: value_type.into(),
      mutability,
      initial,
      span: None,
    }
  }

  pub fn with_span(mut self, span: Option<SourceSpan>) -> Self {
    self.span = span;
    self
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalxImportDecl {
  pub name: Rc<str>,
  pub params: Rc<Vec<CalxBoundaryType>>,
  pub result: Option<CalxBoundaryType>,
  pub span: Option<SourceSpan>,
}

impl CalxImportDecl {
  pub fn new(name: impl Into<Rc<str>>, params: Vec<CalxType>, result: Option<CalxType>) -> Self {
    Self {
      name: name.into(),
      params: Rc::new(params.into_iter().map(CalxBoundaryType::Known).collect()),
      result: result.map(CalxBoundaryType::Known),
      span: None,
    }
  }

  pub fn with_span(mut self, span: Option<SourceSpan>) -> Self {
    self.span = span;
    self
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalxProgram {
  functions: Vec<CalxFunc>,
  globals: Vec<CalxGlobalDecl>,
  imports: Vec<CalxImportDecl>,
}

impl CalxProgram {
  pub fn try_new(
    functions: Vec<CalxFunc>,
    globals: Vec<CalxGlobalDecl>,
    imports: Vec<CalxImportDecl>,
  ) -> Result<Self, CalxProgramError> {
    validate_functions(&functions, &imports)?;
    validate_globals(&globals)?;
    validate_imports(&imports)?;
    Ok(Self {
      functions,
      globals,
      imports,
    })
  }

  pub fn functions(&self) -> &[CalxFunc] {
    &self.functions
  }

  pub fn globals(&self) -> &[CalxGlobalDecl] {
    &self.globals
  }

  pub fn imports(&self) -> &[CalxImportDecl] {
    &self.imports
  }

  pub fn into_parts(self) -> (Vec<CalxFunc>, Vec<CalxGlobalDecl>, Vec<CalxImportDecl>) {
    (self.functions, self.globals, self.imports)
  }
}

/// A strict program whose declarations, typed instruction effects, and
/// lowering have all succeeded.
///
/// Its fields remain private so safe callers cannot mutate executable state
/// after validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedProgram {
  functions: Vec<CalxFunc>,
  globals: Vec<CalxGlobalDecl>,
  imports: Vec<CalxImportDecl>,
}

impl ValidatedProgram {
  pub fn try_from_program(program: CalxProgram) -> Result<Self, CalxProgramError> {
    crate::validate_typed_program(&program).map_err(CalxProgramError::from_validation)?;
    let (mut functions, globals, imports) = program.into_parts();
    crate::vm::lower_typed_functions(&mut functions, &imports).map_err(|message| CalxProgramError::new(message, None, None))?;
    Ok(Self {
      functions,
      globals,
      imports,
    })
  }

  pub fn functions(&self) -> &[CalxFunc] {
    &self.functions
  }

  pub fn globals(&self) -> &[CalxGlobalDecl] {
    &self.globals
  }

  pub fn imports(&self) -> &[CalxImportDecl] {
    &self.imports
  }

  pub fn into_parts(self) -> (Vec<CalxFunc>, Vec<CalxGlobalDecl>, Vec<CalxImportDecl>) {
    (self.functions, self.globals, self.imports)
  }
}

/// Declaration or strict-profile conversion failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalxProgramError {
  pub message: String,
  pub function: Option<Rc<str>>,
  pub span: Option<Box<SourceSpan>>,
  validation: Option<Box<ValidationError>>,
}

impl CalxProgramError {
  pub fn diagnostic(&self) -> DiagnosticView<'_> {
    if let Some(error) = self.validation.as_deref() {
      return error.diagnostic();
    }
    DiagnosticView {
      code: DiagnosticCode::Validation,
      phase: DiagnosticPhase::Validation,
      message: &self.message,
      function: self.function.as_deref(),
      instruction_index: None,
      span: self.span.as_deref(),
      expected_stack: None,
      actual_stack: None,
    }
  }

  pub(crate) fn new(message: impl Into<String>, function: Option<Rc<str>>, span: Option<SourceSpan>) -> Self {
    Self {
      message: message.into(),
      function,
      span: span.map(Box::new),
      validation: None,
    }
  }

  fn from_validation(error: ValidationError) -> Self {
    let message = error.message.clone();
    let function = Some(Rc::from(error.function.as_str()));
    let span = error.span.clone();
    Self {
      message,
      function,
      span,
      validation: Some(Box::new(error)),
    }
  }

  pub fn validation_error(&self) -> Option<&ValidationError> {
    self.validation.as_deref()
  }
}

impl fmt::Display for CalxProgramError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.diagnostic().fmt(f)
  }
}

impl std::error::Error for CalxProgramError {}

/// Typed execution distinguishes void from the explicit `Calx::Nil` value.
#[derive(Debug, Clone, PartialEq)]
pub enum CalxRunResult {
  Void,
  Value(Calx),
}

#[derive(Clone, Copy)]
pub enum CalxHostCallback {
  Void(fn(&[Calx]) -> Result<(), CalxError>),
  Value(fn(&[Calx]) -> Result<Calx, CalxError>),
}

impl fmt::Debug for CalxHostCallback {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Void(_) => f.write_str("CalxHostCallback::Void(..)"),
      Self::Value(_) => f.write_str("CalxHostCallback::Value(..)"),
    }
  }
}

#[derive(Debug, Clone)]
pub struct CalxHostBinding {
  callback: CalxHostCallback,
  params: Rc<Vec<CalxType>>,
  result: Option<CalxType>,
}

impl CalxHostBinding {
  pub fn void(params: Vec<CalxType>, callback: fn(&[Calx]) -> Result<(), CalxError>) -> Result<Self, CalxProgramError> {
    for param in &params {
      validate_strict_type(*param, "host import parameter", None, None)?;
    }
    Ok(Self {
      callback: CalxHostCallback::Void(callback),
      params: Rc::new(params),
      result: None,
    })
  }

  pub fn value(
    params: Vec<CalxType>,
    result: CalxType,
    callback: fn(&[Calx]) -> Result<Calx, CalxError>,
  ) -> Result<Self, CalxProgramError> {
    for param in &params {
      validate_strict_type(*param, "host import parameter", None, None)?;
    }
    validate_strict_type(result, "host import result", None, None)?;
    Ok(Self {
      callback: CalxHostCallback::Value(callback),
      params: Rc::new(params),
      result: Some(result),
    })
  }

  pub fn callback(&self) -> CalxHostCallback {
    self.callback
  }

  pub fn params(&self) -> &[CalxType] {
    &self.params
  }

  pub fn result(&self) -> Option<CalxType> {
    self.result
  }
}

pub type CalxHostBindings = std::collections::HashMap<Rc<str>, CalxHostBinding>;

fn validate_functions(functions: &[CalxFunc], imports: &[CalxImportDecl]) -> Result<(), CalxProgramError> {
  let mut names = HashSet::new();
  let import_names = imports.iter().map(|import| import.name.as_ref()).collect::<HashSet<_>>();

  for function in functions {
    if !names.insert(function.name.as_ref()) {
      return Err(CalxProgramError::new(
        format!("duplicate function declaration `{}`", function.name),
        Some(function.name.clone()),
        None,
      ));
    }
    for value_type in function.params_types.iter().chain(function.ret_types.iter()) {
      validate_strict_type(*value_type, "function boundary", Some(function.name.clone()), None)?;
    }
    let expected_local_names = function.params_types.len() + function.locals.len();
    if function.local_names.len() != expected_local_names {
      return Err(CalxProgramError::new(
        format!(
          "function local metadata mismatch: expected {expected_local_names} names, found {}",
          function.local_names.len()
        ),
        Some(function.name.clone()),
        None,
      ));
    }
    let mut local_names = HashSet::new();
    for name in function.local_names.iter().take(function.params_types.len()) {
      if !local_names.insert(name.as_str()) {
        return Err(CalxProgramError::new(
          format!("duplicate function parameter/local declaration `{name}`"),
          Some(function.name.clone()),
          None,
        ));
      }
    }
    for (index, local) in function.locals.iter().enumerate() {
      if function.local_names[function.params_types.len() + index] != local.name.as_ref() {
        return Err(CalxProgramError::new(
          format!(
            "local declaration `{}` is not aligned with the function local index space",
            local.name
          ),
          Some(function.name.clone()),
          local.span.clone(),
        ));
      }
      if !local_names.insert(local.name.as_ref()) {
        return Err(CalxProgramError::new(
          format!("duplicate function parameter/local declaration `{}`", local.name),
          Some(function.name.clone()),
          local.span.clone(),
        ));
      }
      validate_boundary(
        local.value_type,
        "local",
        local.name.as_ref(),
        Some(function.name.clone()),
        local.span.clone(),
      )?;
    }
    if function
      .syntax
      .iter()
      .any(|syntax| matches!(syntax, CalxSyntax::LocalNew | CalxSyntax::GlobalNew))
    {
      return Err(CalxProgramError::new(
        "strict programs cannot contain legacy local.new/global.new instructions",
        Some(function.name.clone()),
        None,
      ));
    }
    for syntax in function.syntax.iter() {
      if let CalxSyntax::CallImport(name) = syntax {
        if !import_names.contains(name.as_ref()) {
          return Err(CalxProgramError::new(
            format!("strict program calls undeclared import `{name}`"),
            Some(function.name.clone()),
            None,
          ));
        }
      }
    }
  }
  Ok(())
}

fn validate_globals(globals: &[CalxGlobalDecl]) -> Result<(), CalxProgramError> {
  let mut names = HashSet::new();
  for global in globals {
    if !names.insert(global.name.as_ref()) {
      return Err(CalxProgramError::new(
        format!("duplicate global declaration `{}`", global.name),
        None,
        global.span.clone(),
      ));
    }
    validate_boundary(global.value_type, "global", global.name.as_ref(), None, global.span.clone())?;
    let expected = match global.value_type {
      CalxBoundaryType::Known(value_type) => value_type,
      CalxBoundaryType::Dynamic => {
        return Err(CalxProgramError::new(
          format!("strict global `{}` cannot use Dynamic", global.name),
          None,
          global.span.clone(),
        ));
      }
    };
    let actual = global.initial.value_type();
    if expected != actual {
      return Err(CalxProgramError::new(
        format!(
          "global `{}` initializer type mismatch: expected {expected:?}, found {actual:?}",
          global.name
        ),
        None,
        global.span.clone(),
      ));
    }
  }
  Ok(())
}

fn validate_imports(imports: &[CalxImportDecl]) -> Result<(), CalxProgramError> {
  let mut names = HashSet::new();
  for import in imports {
    if !names.insert(import.name.as_ref()) {
      return Err(CalxProgramError::new(
        format!("duplicate import declaration `{}`", import.name),
        None,
        import.span.clone(),
      ));
    }
    for param in import.params.iter() {
      validate_boundary(*param, "import parameter", import.name.as_ref(), None, import.span.clone())?;
    }
    if let Some(result) = import.result {
      validate_boundary(result, "import result", import.name.as_ref(), None, import.span.clone())?;
    }
  }
  Ok(())
}

fn validate_boundary(
  boundary: CalxBoundaryType,
  boundary_kind: &str,
  name: &str,
  function: Option<Rc<str>>,
  span: Option<SourceSpan>,
) -> Result<(), CalxProgramError> {
  match boundary {
    CalxBoundaryType::Known(value_type) => validate_strict_type(value_type, boundary_kind, function, span),
    CalxBoundaryType::Dynamic => Err(CalxProgramError::new(
      format!("strict {boundary_kind} `{name}` cannot use Dynamic"),
      function,
      span,
    )),
  }
}

pub(crate) fn validate_strict_type(
  value_type: CalxType,
  boundary_kind: &str,
  function: Option<Rc<str>>,
  span: Option<SourceSpan>,
) -> Result<(), CalxProgramError> {
  match value_type {
    CalxType::Nil => Err(CalxProgramError::new(
      format!("strict {boundary_kind} cannot use Nil; use a zero-result boundary for void"),
      function,
      span,
    )),
    CalxType::Link => Err(CalxProgramError::new(
      format!("strict {boundary_kind} cannot use Link before it has runtime semantics"),
      function,
      span,
    )),
    CalxType::Bool | CalxType::I64 | CalxType::F64 | CalxType::Str | CalxType::List => Ok(()),
  }
}
