//! Typed, source-aware construction of unvalidated Calx programs.
//!
//! The builder owns declaration indices and flattened structured-control
//! targets, while opaque handles prevent accidental cross-function or
//! cross-module references. It deliberately stops at [`CalxProgram`]: callers
//! must pass the result through [`ValidatedProgram`](crate::ValidatedProgram)
//! or [`CalxVM::from_program`](crate::CalxVM::from_program) before execution.
//!
//! ```
//! use calx_vm::{
//!   Calx, CalxMutability, CalxProgram, CalxType, FunctionBuilder,
//!   ProgramBuilder, ValidatedProgram,
//! };
//!
//! fn generated_program() -> Result<CalxProgram, calx_vm::CalxBuildError> {
//!   let mut program = ProgramBuilder::new();
//!   let answer = program.global(
//!     "$answer",
//!     CalxType::F64,
//!     CalxMutability::Const,
//!     Calx::F64(42.0),
//!   )?;
//!   let mut main = FunctionBuilder::synthetic(
//!     "main",
//!     vec![CalxType::F64],
//!     "generated:calcit/app.main/main",
//!   )?;
//!   main.body().global_get(&answer)?.return_()?;
//!   program.function(main)?;
//!   program.build()
//! }
//!
//! let program = generated_program().expect("well-formed declarations");
//! let _validated = ValidatedProgram::try_from_program(program)
//!   .expect("valid stack and control-flow effects");
//! ```

use core::fmt;
use std::collections::HashSet;
use std::rc::Rc;

use crate::diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticView, SourceSpan};
use crate::program::validate_strict_type;
use crate::{
  Calx, CalxFunc, CalxGlobalDecl, CalxImportDecl, CalxLocalDecl, CalxMutability, CalxProgram, CalxProgramError, CalxSyntax, CalxType,
};

/// Stable category for errors raised while assembling an unvalidated program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalxBuildErrorKind {
  InvalidName,
  InvalidType,
  DuplicateDeclaration,
  InvalidInitializer,
  InvalidInstruction,
  InvalidDeclarationOrder,
  ForeignHandle,
  IndexOverflow,
  ProgramContract,
}

/// Structured failure returned by [`ProgramBuilder`], [`FunctionBuilder`], or
/// [`BodyBuilder`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalxBuildError {
  pub kind: CalxBuildErrorKind,
  pub message: String,
  pub function: Option<Rc<str>>,
  pub span: Option<Box<SourceSpan>>,
}

impl CalxBuildError {
  fn new(kind: CalxBuildErrorKind, message: impl Into<String>, function: Option<Rc<str>>, span: Option<SourceSpan>) -> Self {
    Self {
      kind,
      message: message.into(),
      function,
      span: span.map(Box::new),
    }
  }

  fn from_program(error: CalxProgramError) -> Self {
    Self {
      kind: CalxBuildErrorKind::ProgramContract,
      message: error.message,
      function: error.function,
      span: error.span,
    }
  }

  /// Returns the common diagnostic representation used by parse, validation,
  /// runtime, and host failures.
  pub fn diagnostic(&self) -> DiagnosticView<'_> {
    DiagnosticView {
      code: DiagnosticCode::ProgramBuild,
      phase: DiagnosticPhase::Build,
      message: &self.message,
      function: self.function.as_deref(),
      instruction_index: None,
      span: self.span.as_deref(),
      expected_stack: None,
      actual_stack: None,
    }
  }
}

impl fmt::Display for CalxBuildError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.diagnostic().fmt(f)
  }
}

impl std::error::Error for CalxBuildError {}

/// Opaque index of a parameter or declared local in one function.
#[derive(Debug, Clone)]
pub struct LocalId {
  index: usize,
  value_type: CalxType,
  owner: Rc<()>,
}

impl LocalId {
  pub fn index(&self) -> usize {
    self.index
  }

  pub fn value_type(&self) -> CalxType {
    self.value_type
  }
}

/// Opaque index of a declared module global.
#[derive(Debug, Clone)]
pub struct GlobalId {
  index: usize,
  value_type: CalxType,
  mutability: CalxMutability,
  owner: Rc<()>,
}

impl GlobalId {
  pub fn index(&self) -> usize {
    self.index
  }

  pub fn value_type(&self) -> CalxType {
    self.value_type
  }

  pub fn mutability(&self) -> CalxMutability {
    self.mutability
  }
}

/// Opaque reference to a declared typed host import.
#[derive(Debug, Clone)]
pub struct ImportId {
  name: Rc<str>,
  params: Rc<Vec<CalxType>>,
  result: Option<CalxType>,
  owner: Rc<()>,
}

impl ImportId {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn params(&self) -> &[CalxType] {
    &self.params
  }

  pub fn result(&self) -> Option<CalxType> {
    self.result
  }
}

/// Builds module declarations and functions without generating Cirru text.
///
/// `build()` returns an unvalidated [`CalxProgram`]. Consumers must still use
/// `ValidatedProgram::try_from_program` or `CalxVM::from_program`; the builder
/// never constructs lowered executable state.
#[derive(Debug, Default)]
pub struct ProgramBuilder {
  functions: Vec<CalxFunc>,
  globals: Vec<CalxGlobalDecl>,
  imports: Vec<CalxImportDecl>,
  function_names: HashSet<Rc<str>>,
  global_names: HashSet<Rc<str>>,
  import_names: HashSet<Rc<str>>,
  owner: Rc<()>,
}

impl ProgramBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  /// Adds a strict global without source metadata.
  pub fn global(
    &mut self,
    name: impl Into<Rc<str>>,
    value_type: CalxType,
    mutability: CalxMutability,
    initial: Calx,
  ) -> Result<GlobalId, CalxBuildError> {
    self.global_at(name, value_type, mutability, initial, None)
  }

  /// Adds a strict global with a source-backed or synthetic origin.
  pub fn global_at(
    &mut self,
    name: impl Into<Rc<str>>,
    value_type: CalxType,
    mutability: CalxMutability,
    initial: Calx,
    span: Option<SourceSpan>,
  ) -> Result<GlobalId, CalxBuildError> {
    let name = checked_name(name, "global", None, span.clone())?;
    if value_type == CalxType::F64Buffer {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidType,
        "strict global cannot use F64Buffer; pass immutable buffers through function or import boundaries",
        None,
        span,
      ));
    }
    validate_builder_type(value_type, "global", None, span.clone())?;
    if initial.value_type() != value_type {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidInitializer,
        format!(
          "global `{name}` initializer type mismatch: expected {value_type:?}, found {:?}",
          initial.value_type()
        ),
        None,
        span,
      ));
    }
    if !self.global_names.insert(name.clone()) {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::DuplicateDeclaration,
        format!("duplicate global declaration `{name}`"),
        None,
        span,
      ));
    }

    let id = GlobalId {
      index: self.globals.len(),
      value_type,
      mutability,
      owner: self.owner.clone(),
    };
    self
      .globals
      .push(CalxGlobalDecl::new(name, value_type, mutability, initial).with_span(span));
    Ok(id)
  }

  /// Adds a zero- or single-result strict host import without source metadata.
  pub fn import(
    &mut self,
    name: impl Into<Rc<str>>,
    params: Vec<CalxType>,
    result: Option<CalxType>,
  ) -> Result<ImportId, CalxBuildError> {
    self.import_at(name, params, result, None)
  }

  /// Adds a zero- or single-result strict host import with an origin.
  pub fn import_at(
    &mut self,
    name: impl Into<Rc<str>>,
    params: Vec<CalxType>,
    result: Option<CalxType>,
    span: Option<SourceSpan>,
  ) -> Result<ImportId, CalxBuildError> {
    let name = checked_name(name, "import", None, span.clone())?;
    for value_type in &params {
      validate_builder_type(*value_type, "import parameter", None, span.clone())?;
    }
    if let Some(value_type) = result {
      validate_builder_type(value_type, "import result", None, span.clone())?;
    }
    if !self.import_names.insert(name.clone()) {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::DuplicateDeclaration,
        format!("duplicate import declaration `{name}`"),
        None,
        span,
      ));
    }

    let id = ImportId {
      name: name.clone(),
      params: Rc::new(params.clone()),
      result,
      owner: self.owner.clone(),
    };
    self.imports.push(CalxImportDecl::new(name, params, result).with_span(span));
    Ok(id)
  }

  /// Finishes and adds one typed function.
  pub fn function(&mut self, function: FunctionBuilder) -> Result<(), CalxBuildError> {
    if function
      .body
      .program_owner
      .as_ref()
      .is_some_and(|owner| !Rc::ptr_eq(owner, &self.owner))
    {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::ForeignHandle,
        "function uses a global or import handle from another ProgramBuilder",
        Some(function.name.clone()),
        function.body.default_span.clone(),
      ));
    }
    let function = function.build();
    if !self.function_names.insert(function.name.clone()) {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::DuplicateDeclaration,
        format!("duplicate function declaration `{}`", function.name),
        Some(function.name.clone()),
        function.source_spans.first().cloned().flatten(),
      ));
    }
    self.functions.push(function);
    Ok(())
  }

  /// Produces an unvalidated strict program declaration graph.
  pub fn build(self) -> Result<CalxProgram, CalxBuildError> {
    CalxProgram::try_new(self.functions, self.globals, self.imports).map_err(CalxBuildError::from_program)
  }
}

/// Builds a typed function signature, local declarations, and source body.
pub struct FunctionBuilder {
  name: Rc<str>,
  params: Vec<CalxType>,
  results: Vec<CalxType>,
  locals: Vec<CalxLocalDecl>,
  local_names: Vec<String>,
  local_name_set: HashSet<Rc<str>>,
  body: BodyBuilder,
  local_owner: Rc<()>,
}

impl fmt::Debug for FunctionBuilder {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("FunctionBuilder")
      .field("name", &self.name)
      .field("params", &self.params)
      .field("results", &self.results)
      .field("locals", &self.locals)
      .field("body", &self.body)
      .finish()
  }
}

impl FunctionBuilder {
  /// Starts a function with no parameters and the specified result types.
  pub fn new(name: impl Into<Rc<str>>, results: Vec<CalxType>) -> Result<Self, CalxBuildError> {
    let name = checked_name(name, "function", None, None)?;
    for result in &results {
      validate_builder_type(*result, "function result", Some(name.clone()), None)?;
    }
    let local_owner = Rc::new(());
    Ok(Self {
      body: BodyBuilder::new(name.clone(), local_owner.clone(), None, None),
      name,
      params: vec![],
      results,
      locals: vec![],
      local_names: vec![],
      local_name_set: HashSet::new(),
      local_owner,
    })
  }

  /// Starts a generated function whose declarations and instructions default
  /// to a stable synthetic `source:1:1` origin.
  pub fn synthetic(name: impl Into<Rc<str>>, results: Vec<CalxType>, source: impl Into<Rc<str>>) -> Result<Self, CalxBuildError> {
    Ok(Self::new(name, results)?.with_default_span(SourceSpan::synthetic(source)))
  }

  /// Sets the default origin for subsequently declared locals and emitted
  /// instructions. `emit_at` and `local_at` can override it per item.
  pub fn with_default_span(mut self, span: SourceSpan) -> Self {
    self.body.default_span = Some(span);
    self
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  /// Adds a named typed parameter and returns its local-space handle.
  pub fn parameter(&mut self, name: impl Into<Rc<str>>, value_type: CalxType) -> Result<LocalId, CalxBuildError> {
    self.ensure_declarations_open("parameter")?;
    let name = checked_name(name, "parameter", Some(self.name.clone()), self.body.default_span.clone())?;
    validate_builder_type(
      value_type,
      "function parameter",
      Some(self.name.clone()),
      self.body.default_span.clone(),
    )?;
    self.insert_unique_local(name.clone(), self.body.default_span.clone())?;
    let id = LocalId {
      index: self.params.len(),
      value_type,
      owner: self.local_owner.clone(),
    };
    self.params.push(value_type);
    self.local_names.push(name.to_string());
    Ok(id)
  }

  /// Adds a declared local using the function's default origin.
  pub fn local(&mut self, name: impl Into<Rc<str>>, value_type: CalxType) -> Result<LocalId, CalxBuildError> {
    self.local_at(name, value_type, self.body.default_span.clone())
  }

  /// Adds a declared local with an explicit source-backed or synthetic origin.
  pub fn local_at(
    &mut self,
    name: impl Into<Rc<str>>,
    value_type: CalxType,
    span: Option<SourceSpan>,
  ) -> Result<LocalId, CalxBuildError> {
    self.ensure_declarations_open("local")?;
    let name = checked_name(name, "local", Some(self.name.clone()), span.clone())?;
    validate_builder_type(value_type, "local", Some(self.name.clone()), span.clone())?;
    self.insert_unique_local(name.clone(), span.clone())?;
    let id = LocalId {
      index: self.params.len() + self.locals.len(),
      value_type,
      owner: self.local_owner.clone(),
    };
    self.locals.push(CalxLocalDecl::new(name.clone(), value_type).with_span(span));
    self.local_names.push(name.to_string());
    Ok(id)
  }

  pub fn body(&mut self) -> &mut BodyBuilder {
    &mut self.body
  }

  /// Finishes the function as source syntax. It is still unvalidated and has
  /// no lowered instructions.
  pub fn build(self) -> CalxFunc {
    CalxFunc::new(self.name, self.params, self.results, self.body.syntax)
      .with_locals(self.locals)
      .with_local_names(self.local_names)
      .with_source_spans(self.body.spans)
  }

  fn ensure_declarations_open(&self, declaration: &str) -> Result<(), CalxBuildError> {
    if self.body.syntax.is_empty() {
      Ok(())
    } else {
      Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidDeclarationOrder,
        format!("{declaration} declarations must be added before the first instruction"),
        Some(self.name.clone()),
        self.body.default_span.clone(),
      ))
    }
  }

  fn insert_unique_local(&mut self, name: Rc<str>, span: Option<SourceSpan>) -> Result<(), CalxBuildError> {
    if !self.local_name_set.insert(name.clone()) {
      Err(CalxBuildError::new(
        CalxBuildErrorKind::DuplicateDeclaration,
        format!("duplicate function parameter/local declaration `{name}`"),
        Some(self.name.clone()),
        span,
      ))
    } else {
      Ok(())
    }
  }
}

/// Builds a flat `CalxSyntax` body while owning structural target calculation.
#[derive(Debug)]
pub struct BodyBuilder {
  function: Rc<str>,
  syntax: Vec<CalxSyntax>,
  spans: Vec<Option<SourceSpan>>,
  default_span: Option<SourceSpan>,
  local_owner: Rc<()>,
  program_owner: Option<Rc<()>>,
}

impl BodyBuilder {
  fn new(function: Rc<str>, local_owner: Rc<()>, program_owner: Option<Rc<()>>, default_span: Option<SourceSpan>) -> Self {
    Self {
      function,
      syntax: vec![],
      spans: vec![],
      default_span,
      local_owner,
      program_owner,
    }
  }

  pub fn len(&self) -> usize {
    self.syntax.len()
  }

  pub fn is_empty(&self) -> bool {
    self.syntax.is_empty()
  }

  /// Emits one non-structural instruction with the default origin.
  pub fn emit(&mut self, syntax: CalxSyntax) -> Result<&mut Self, CalxBuildError> {
    self.emit_with_span(syntax, self.default_span.clone())
  }

  /// Emits one non-structural instruction with an explicit origin.
  pub fn emit_at(&mut self, syntax: CalxSyntax, span: SourceSpan) -> Result<&mut Self, CalxBuildError> {
    self.emit_with_span(syntax, Some(span))
  }

  pub fn constant(&mut self, value: Calx) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::Const(value))
  }

  /// Emit `f64-buffer.len` using the current source origin.
  pub fn f64_buffer_len(&mut self) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::F64BufferLen)
  }

  /// Emit the checked `f64.to-i64-index` conversion.
  pub fn f64_to_i64_index(&mut self) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::F64ToI64Index)
  }

  /// Emit `f64-buffer.get` using the current source origin.
  pub fn f64_buffer_get(&mut self) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::F64BufferGet)
  }

  pub fn local_get(&mut self, local: &LocalId) -> Result<&mut Self, CalxBuildError> {
    self.ensure_local_owner(local)?;
    self.push(CalxSyntax::LocalGet(local.index), self.default_span.clone());
    Ok(self)
  }

  pub fn local_set(&mut self, local: &LocalId) -> Result<&mut Self, CalxBuildError> {
    self.ensure_local_owner(local)?;
    self.push(CalxSyntax::LocalSet(local.index), self.default_span.clone());
    Ok(self)
  }

  pub fn local_tee(&mut self, local: &LocalId) -> Result<&mut Self, CalxBuildError> {
    self.ensure_local_owner(local)?;
    self.push(CalxSyntax::LocalTee(local.index), self.default_span.clone());
    Ok(self)
  }

  pub fn global_get(&mut self, global: &GlobalId) -> Result<&mut Self, CalxBuildError> {
    self.ensure_program_owner(&global.owner)?;
    self.push(CalxSyntax::GlobalGet(global.index), self.default_span.clone());
    Ok(self)
  }

  pub fn global_set(&mut self, global: &GlobalId) -> Result<&mut Self, CalxBuildError> {
    if global.mutability == CalxMutability::Const {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidInstruction,
        format!("cannot write const global index {}", global.index),
        Some(self.function.clone()),
        self.default_span.clone(),
      ));
    }
    self.ensure_program_owner(&global.owner)?;
    self.push(CalxSyntax::GlobalSet(global.index), self.default_span.clone());
    Ok(self)
  }

  pub fn call(&mut self, function: impl Into<Rc<str>>) -> Result<&mut Self, CalxBuildError> {
    let function = checked_name(function, "called function", Some(self.function.clone()), self.default_span.clone())?;
    self.emit(CalxSyntax::Call(function))
  }

  pub fn return_call(&mut self, function: impl Into<Rc<str>>) -> Result<&mut Self, CalxBuildError> {
    let function = checked_name(
      function,
      "tail-called function",
      Some(self.function.clone()),
      self.default_span.clone(),
    )?;
    self.emit(CalxSyntax::ReturnCall(function))
  }

  pub fn call_import(&mut self, import: &ImportId) -> Result<&mut Self, CalxBuildError> {
    self.ensure_program_owner(&import.owner)?;
    self.push(CalxSyntax::CallImport(import.name.clone()), self.default_span.clone());
    Ok(self)
  }

  pub fn branch(&mut self, depth: usize) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::Br(depth))
  }

  pub fn branch_if(&mut self, depth: usize) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::BrIf(depth))
  }

  pub fn return_(&mut self) -> Result<&mut Self, CalxBuildError> {
    self.emit(CalxSyntax::Return)
  }

  /// Changes the origin used by subsequent instructions without allocating a
  /// temporary instruction node. Pass `None` for intentionally unknown source.
  pub fn set_default_span(&mut self, span: Option<SourceSpan>) -> &mut Self {
    self.default_span = span;
    self
  }

  /// Builds a structured block. The child body is committed only after its
  /// closure succeeds.
  pub fn block<F>(&mut self, params: Vec<CalxType>, results: Vec<CalxType>, build: F) -> Result<&mut Self, CalxBuildError>
  where
    F: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    self.block_at(params, results, self.default_span.clone(), build)
  }

  pub fn block_at<F>(
    &mut self,
    params: Vec<CalxType>,
    results: Vec<CalxType>,
    span: Option<SourceSpan>,
    build: F,
  ) -> Result<&mut Self, CalxBuildError>
  where
    F: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    self.structured_block(false, params, results, span, build)
  }

  /// Builds a structured loop with the same all-or-nothing closure behavior as
  /// [`BodyBuilder::block`].
  pub fn loop_<F>(&mut self, params: Vec<CalxType>, results: Vec<CalxType>, build: F) -> Result<&mut Self, CalxBuildError>
  where
    F: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    self.loop_at(params, results, self.default_span.clone(), build)
  }

  pub fn loop_at<F>(
    &mut self,
    params: Vec<CalxType>,
    results: Vec<CalxType>,
    span: Option<SourceSpan>,
    build: F,
  ) -> Result<&mut Self, CalxBuildError>
  where
    F: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    self.structured_block(true, params, results, span, build)
  }

  /// Builds an if/else using the parser's canonical flattened branch order.
  /// Neither branch is committed when either closure returns an error.
  pub fn if_else<Then, Else>(&mut self, results: Vec<CalxType>, build_then: Then, build_else: Else) -> Result<&mut Self, CalxBuildError>
  where
    Then: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
    Else: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    self.if_else_at(results, self.default_span.clone(), build_then, build_else)
  }

  pub fn if_else_at<Then, Else>(
    &mut self,
    results: Vec<CalxType>,
    span: Option<SourceSpan>,
    build_then: Then,
    build_else: Else,
  ) -> Result<&mut Self, CalxBuildError>
  where
    Then: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
    Else: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    validate_builder_types(&results, "if result", Some(self.function.clone()), span.clone())?;
    let mut then_body = BodyBuilder::new(
      self.function.clone(),
      self.local_owner.clone(),
      self.program_owner.clone(),
      self.default_span.clone(),
    );
    build_then(&mut then_body)?;
    let mut else_body = BodyBuilder::new(
      self.function.clone(),
      self.local_owner.clone(),
      self.program_owner.clone(),
      self.default_span.clone(),
    );
    build_else(&mut else_body)?;

    let else_at = checked_sum(&[else_body.len(), 2], &self.function, span.clone())?;
    let to = checked_sum(&[else_body.len(), then_body.len(), 3], &self.function, span.clone())?;
    let mut section = BodyBuilder::new(
      self.function.clone(),
      self.local_owner.clone(),
      self.program_owner.clone(),
      self.default_span.clone(),
    );
    section.push(
      CalxSyntax::If {
        ret_types: Rc::new(results),
        else_at,
        to,
      },
      span.clone(),
    );
    section.append_rebased(else_body)?;
    section.push(CalxSyntax::ElseEnd, span.clone());
    section.append_rebased(then_body)?;
    section.push(CalxSyntax::ThenEnd, span);
    self.append_rebased(section)?;
    Ok(self)
  }

  fn structured_block<F>(
    &mut self,
    looped: bool,
    params: Vec<CalxType>,
    results: Vec<CalxType>,
    span: Option<SourceSpan>,
    build: F,
  ) -> Result<&mut Self, CalxBuildError>
  where
    F: FnOnce(&mut BodyBuilder) -> Result<(), CalxBuildError>,
  {
    validate_builder_types(&params, "control parameter", Some(self.function.clone()), span.clone())?;
    validate_builder_types(&results, "control result", Some(self.function.clone()), span.clone())?;
    let mut body = BodyBuilder::new(
      self.function.clone(),
      self.local_owner.clone(),
      self.program_owner.clone(),
      self.default_span.clone(),
    );
    build(&mut body)?;
    let to = checked_sum(&[body.len(), 1], &self.function, span.clone())?;
    let mut section = BodyBuilder::new(
      self.function.clone(),
      self.local_owner.clone(),
      self.program_owner.clone(),
      self.default_span.clone(),
    );
    section.push(
      CalxSyntax::Block {
        looped,
        params_types: Rc::new(params),
        ret_types: Rc::new(results),
        from: 1,
        to,
      },
      span.clone(),
    );
    section.append_rebased(body)?;
    section.push(CalxSyntax::BlockEnd(looped), span);
    self.append_rebased(section)?;
    Ok(self)
  }

  fn emit_with_span(&mut self, syntax: CalxSyntax, span: Option<SourceSpan>) -> Result<&mut Self, CalxBuildError> {
    if let CalxSyntax::Const(value) = &syntax {
      validate_builder_type(value.value_type(), "constant", Some(self.function.clone()), span.clone())?;
    }
    if matches!(&syntax, CalxSyntax::Const(Calx::F64Buffer(_))) {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidInstruction,
        "F64Buffer constants are not part of the initial typed-buffer syntax; use an entry or import value",
        Some(self.function.clone()),
        span,
      ));
    }
    if matches!(
      syntax,
      CalxSyntax::LocalSet(_)
        | CalxSyntax::LocalTee(_)
        | CalxSyntax::LocalGet(_)
        | CalxSyntax::LocalNew
        | CalxSyntax::GlobalSet(_)
        | CalxSyntax::GlobalGet(_)
        | CalxSyntax::GlobalNew
        | CalxSyntax::CallImport(_)
        | CalxSyntax::Block { .. }
        | CalxSyntax::BlockEnd(_)
        | CalxSyntax::Do(_)
        | CalxSyntax::If { .. }
        | CalxSyntax::ElseEnd
        | CalxSyntax::ThenEnd
    ) {
      return Err(CalxBuildError::new(
        CalxBuildErrorKind::InvalidInstruction,
        format!("instruction {syntax:?} must be produced by a typed builder method"),
        Some(self.function.clone()),
        span,
      ));
    }
    self.push(syntax, span);
    Ok(self)
  }

  fn push(&mut self, syntax: CalxSyntax, span: Option<SourceSpan>) {
    self.syntax.push(syntax);
    self.spans.push(span);
  }

  fn append_rebased(&mut self, mut child: BodyBuilder) -> Result<(), CalxBuildError> {
    if let Some(owner) = &child.program_owner {
      self.check_program_owner(owner)?;
    }
    let offset = self.syntax.len();
    for syntax in &mut child.syntax {
      rebase_syntax(syntax, offset, &self.function, child.default_span.clone())?;
    }
    if self.program_owner.is_none() {
      self.program_owner = child.program_owner.clone();
    }
    self.syntax.extend(child.syntax);
    self.spans.extend(child.spans);
    Ok(())
  }

  fn ensure_local_owner(&self, local: &LocalId) -> Result<(), CalxBuildError> {
    if Rc::ptr_eq(&self.local_owner, &local.owner) {
      Ok(())
    } else {
      Err(CalxBuildError::new(
        CalxBuildErrorKind::ForeignHandle,
        "local handle belongs to another FunctionBuilder",
        Some(self.function.clone()),
        self.default_span.clone(),
      ))
    }
  }

  fn ensure_program_owner(&mut self, owner: &Rc<()>) -> Result<(), CalxBuildError> {
    self.check_program_owner(owner)?;
    if self.program_owner.is_none() {
      self.program_owner = Some(owner.clone());
    }
    Ok(())
  }

  fn check_program_owner(&self, owner: &Rc<()>) -> Result<(), CalxBuildError> {
    match &self.program_owner {
      Some(current) if !Rc::ptr_eq(current, owner) => Err(CalxBuildError::new(
        CalxBuildErrorKind::ForeignHandle,
        "global/import handles from different ProgramBuilder instances cannot be mixed",
        Some(self.function.clone()),
        self.default_span.clone(),
      )),
      Some(_) | None => Ok(()),
    }
  }
}

fn checked_name(
  name: impl Into<Rc<str>>,
  kind: &str,
  function: Option<Rc<str>>,
  span: Option<SourceSpan>,
) -> Result<Rc<str>, CalxBuildError> {
  let name = name.into();
  if name.is_empty() {
    Err(CalxBuildError::new(
      CalxBuildErrorKind::InvalidName,
      format!("{kind} name cannot be empty"),
      function,
      span,
    ))
  } else {
    Ok(name)
  }
}

fn validate_builder_type(
  value_type: CalxType,
  boundary: &str,
  function: Option<Rc<str>>,
  span: Option<SourceSpan>,
) -> Result<(), CalxBuildError> {
  validate_strict_type(value_type, boundary, function.clone(), span.clone()).map_err(|error| {
    CalxBuildError::new(
      CalxBuildErrorKind::InvalidType,
      error.message,
      error.function.or(function),
      error.span.map(|value| *value).or(span),
    )
  })
}

fn validate_builder_types(
  value_types: &[CalxType],
  boundary: &str,
  function: Option<Rc<str>>,
  span: Option<SourceSpan>,
) -> Result<(), CalxBuildError> {
  for value_type in value_types {
    validate_builder_type(*value_type, boundary, function.clone(), span.clone())?;
  }
  Ok(())
}

fn checked_sum(parts: &[usize], function: &Rc<str>, span: Option<SourceSpan>) -> Result<usize, CalxBuildError> {
  parts.iter().try_fold(0usize, |sum, part| {
    sum.checked_add(*part).ok_or_else(|| {
      CalxBuildError::new(
        CalxBuildErrorKind::IndexOverflow,
        "structured instruction index overflow",
        Some(function.clone()),
        span.clone(),
      )
    })
  })
}

fn rebase_syntax(syntax: &mut CalxSyntax, offset: usize, function: &Rc<str>, span: Option<SourceSpan>) -> Result<(), CalxBuildError> {
  match syntax {
    CalxSyntax::Block { from, to, .. } => {
      *from = checked_sum(&[*from, offset], function, span.clone())?;
      *to = checked_sum(&[*to, offset], function, span)?;
    }
    CalxSyntax::If { else_at, to, .. } => {
      *else_at = checked_sum(&[*else_at, offset], function, span.clone())?;
      *to = checked_sum(&[*to, offset], function, span)?;
    }
    _ => {}
  }
  Ok(())
}
