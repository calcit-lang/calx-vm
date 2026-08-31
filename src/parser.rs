/*! Parse Cirru into Calx instructions
 *
 */

mod locals;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use cirru_parser::{parse, Cirru, CirruError};

use crate::calx::CalxType;
use crate::diagnostic::{DiagnosticCode, DiagnosticPhase, DiagnosticView, SourcePosition, SourceSpan};
use crate::program::{CalxBoundaryType, CalxGlobalDecl, CalxImportDecl, CalxLocalDecl, CalxMutability, CalxProgram, CalxProgramError};
use crate::syntax::CalxSyntax;
use crate::vm::func::CalxFunc;

use self::locals::LocalsCollector;

/// Source-aware result of parsing a complete Calx file.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgram {
  /// Function AST nodes only, parallel to `functions`.
  pub nodes: Vec<Cirru>,
  pub functions: Vec<CalxFunc>,
  pub globals: Vec<CalxGlobalDecl>,
  pub imports: Vec<CalxImportDecl>,
}

impl ParsedProgram {
  /// Convert parsed metadata into the strict typed profile.
  pub fn into_program(self) -> Result<CalxProgram, CalxProgramError> {
    CalxProgram::try_new(self.functions, self.globals, self.imports)
  }

  /// Count boundaries that force the legacy profile.
  pub fn dynamic_boundary_count(&self) -> usize {
    let local_count = self
      .functions
      .iter()
      .flat_map(|function| function.locals.iter())
      .filter(|local| local.value_type == CalxBoundaryType::Dynamic)
      .count();
    let global_count = self
      .globals
      .iter()
      .filter(|global| global.value_type == CalxBoundaryType::Dynamic)
      .count();
    let import_count = self
      .imports
      .iter()
      .map(|import| {
        import
          .params
          .iter()
          .filter(|value_type| **value_type == CalxBoundaryType::Dynamic)
          .count()
          + usize::from(import.result == Some(CalxBoundaryType::Dynamic))
      })
      .sum::<usize>();
    let declared_imports = self.imports.iter().map(|import| import.name.as_ref()).collect::<HashSet<_>>();
    let mut undeclared_imports = HashSet::new();
    let legacy_global_count = self
      .functions
      .iter()
      .flat_map(|function| function.syntax.iter())
      .filter(|syntax| match syntax {
        CalxSyntax::GlobalNew => true,
        CalxSyntax::CallImport(name) if !declared_imports.contains(name.as_ref()) => {
          undeclared_imports.insert(name.as_ref());
          false
        }
        _ => false,
      })
      .count();
    local_count + global_count + import_count + legacy_global_count + undeclared_imports.len()
  }
}

/// Parse failure with a stable code and optional source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
  pub code: DiagnosticCode,
  pub message: String,
  pub function: Option<String>,
  pub instruction_index: Option<usize>,
  pub span: Option<Box<SourceSpan>>,
}

impl ParseError {
  pub fn diagnostic(&self) -> DiagnosticView<'_> {
    DiagnosticView {
      code: self.code,
      phase: DiagnosticPhase::Parse,
      message: &self.message,
      function: self.function.as_deref(),
      instruction_index: self.instruction_index,
      span: self.span.as_deref(),
      expected_stack: None,
      actual_stack: None,
    }
  }
}

impl core::fmt::Display for ParseError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    self.diagnostic().fmt(f)
  }
}

impl std::error::Error for ParseError {}

/// Parses a complete source file and attaches deterministic source spans to all
/// expanded syntax instructions.
pub fn parse_program(source_name: impl Into<Rc<str>>, code: &str) -> Result<ParsedProgram, ParseError> {
  let source_name = source_name.into();
  let parsed_nodes = parse(code).map_err(|error| cirru_parse_error(source_name.clone(), error))?;
  let tokens = scan_source_tokens(source_name.clone(), code);
  let mut cursor = 0;
  let located = parsed_nodes
    .iter()
    .map(|node| locate_node(node, &tokens, &mut cursor))
    .collect::<Result<Vec<_>, _>>()?;

  let mut function_indices = vec![];
  let mut globals = vec![];
  let mut imports = vec![];
  let mut global_names = HashMap::new();
  let mut import_names = HashMap::new();

  for (index, (node, located_node)) in parsed_nodes.iter().zip(&located).enumerate() {
    let Cirru::List(items) = node else {
      return Err(ParseError {
        code: DiagnosticCode::InstructionParse,
        message: "expected top-level function, global, or import-fn expression".to_string(),
        function: None,
        instruction_index: None,
        span: located_node.span.clone().map(Box::new),
      });
    };
    match items.first().and_then(leaf_value) {
      Some("fn" | "defn") => function_indices.push(index),
      Some("global") => {
        let declaration = parse_global_decl(items, located_node.span.clone()).map_err(|message| ParseError {
          code: DiagnosticCode::InstructionParse,
          message,
          function: None,
          instruction_index: None,
          span: located_node.head_span().or_else(|| located_node.span.clone()).map(Box::new),
        })?;
        if global_names.insert(declaration.name.to_string(), globals.len()).is_some() {
          return Err(ParseError {
            code: DiagnosticCode::InstructionParse,
            message: format!("duplicate global declaration `{}`", declaration.name),
            function: None,
            instruction_index: None,
            span: declaration.span.clone().map(Box::new),
          });
        }
        globals.push(declaration);
      }
      Some("import-fn") => {
        let declaration = parse_import_decl(items, located_node.span.clone()).map_err(|message| ParseError {
          code: DiagnosticCode::InstructionParse,
          message,
          function: None,
          instruction_index: None,
          span: located_node.head_span().or_else(|| located_node.span.clone()).map(Box::new),
        })?;
        if import_names.insert(declaration.name.to_string(), imports.len()).is_some() {
          return Err(ParseError {
            code: DiagnosticCode::InstructionParse,
            message: format!("duplicate import declaration `{}`", declaration.name),
            function: None,
            instruction_index: None,
            span: declaration.span.clone().map(Box::new),
          });
        }
        imports.push(declaration);
      }
      Some(name) => {
        return Err(ParseError {
          code: DiagnosticCode::InstructionParse,
          message: format!("unknown top-level declaration `{name}`"),
          function: None,
          instruction_index: None,
          span: located_node.head_span().or_else(|| located_node.span.clone()).map(Box::new),
        });
      }
      None => {
        return Err(ParseError {
          code: DiagnosticCode::InstructionParse,
          message: "expected a top-level declaration name".to_string(),
          function: None,
          instruction_index: None,
          span: located_node.span.clone().map(Box::new),
        });
      }
    }
  }

  let mut nodes = Vec::with_capacity(function_indices.len());
  let mut functions = Vec::with_capacity(function_indices.len());
  for index in function_indices {
    let node = &parsed_nodes[index];
    let located_node = &located[index];
    let Cirru::List(items) = node else {
      continue;
    };
    let function_name = items.get(1).and_then(|item| match item {
      Cirru::Leaf(name) => Some(name.to_string()),
      Cirru::List(_) => None,
    });
    let mut function = match parse_function_with_globals(items, &global_names) {
      Ok(function) => function,
      Err(message) => {
        let (instruction_index, span) = locate_function_parse_failure(items, located_node, &global_names);
        return Err(ParseError {
          code: DiagnosticCode::InstructionParse,
          message,
          function: function_name,
          instruction_index,
          span: span.map(Box::new),
        });
      }
    };
    attach_local_source_spans(&mut function, located_node);
    let source_spans = function_source_spans(located_node);
    if source_spans.len() != function.syntax.len() {
      return Err(ParseError {
        code: DiagnosticCode::InstructionParse,
        message: format!(
          "internal source mapping mismatch: {} spans for {} syntax instructions",
          source_spans.len(),
          function.syntax.len()
        ),
        function: Some(function.name.to_string()),
        instruction_index: None,
        span: located_node.span.clone().map(Box::new),
      });
    }
    function.source_spans = Rc::new(source_spans);
    nodes.push(node.clone());
    functions.push(function);
  }

  Ok(ParsedProgram {
    nodes,
    functions,
    globals,
    imports,
  })
}

/// parses
/// ```cirru
/// fn <f-name> (i64 f64)
///   const 1
///   echo
/// ```
pub fn parse_function(nodes: &[Cirru]) -> Result<CalxFunc, String> {
  parse_function_with_globals(nodes, &HashMap::new())
}

fn parse_function_with_globals(nodes: &[Cirru], global_names: &HashMap<String, usize>) -> Result<CalxFunc, String> {
  if nodes.len() <= 3 {
    return Err(String::from("function expects at least 3 lines"));
  }

  if !leaf_is(&nodes[0], "fn") && !leaf_is(&nodes[0], "defn") {
    return Err(String::from("Not a function"));
  }

  let name: Rc<str> = if let Cirru::Leaf(x) = &nodes[1] {
    (**x).into()
  } else {
    return Err(String::from("invalid name"));
  };

  let mut body: Vec<CalxSyntax> = vec![];
  let mut local_declarations = vec![];
  let mut locals_collector: LocalsCollector = LocalsCollector::new();

  let (params_types, ret_types) = parse_fn_types(&nodes[2], &mut locals_collector)?;

  let mut ptr_base: usize = 0;
  let mut executable_started = false;
  for line in nodes.iter().skip(3) {
    match instruction_head(line) {
      Some("local") if !executable_started => {
        local_declarations.push(parse_local_decl(line, &mut locals_collector)?);
      }
      Some("local.new") if !executable_started => {
        local_declarations.push(parse_legacy_local_decl(line, &mut locals_collector)?);
        body.push(CalxSyntax::LocalNew);
        ptr_base += 1;
      }
      Some("local" | "local.new") => {
        return Err("local declarations must appear before the first executable instruction".to_string());
      }
      Some(";;") if !executable_started => {}
      _ => {
        executable_started = true;
        for expanded in extract_nested(line)? {
          let syntax = parse_instr(ptr_base, &expanded, &mut locals_collector, global_names)?;

          for instr in syntax {
            ptr_base += 1;
            body.push(instr);
          }
        }
      }
    }
  }

  Ok(
    CalxFunc::new(name, params_types, ret_types, body)
      .with_locals(local_declarations)
      .with_local_names(locals_collector.locals),
  )
}

fn leaf_value(node: &Cirru) -> Option<&str> {
  match node {
    Cirru::Leaf(value) => Some(value),
    Cirru::List(_) => None,
  }
}

fn instruction_head(node: &Cirru) -> Option<&str> {
  match node {
    Cirru::List(items) => items.first().and_then(leaf_value),
    Cirru::Leaf(_) => None,
  }
}

fn parse_decl_name(node: &Cirru, declaration: &str, require_dollar: bool) -> Result<Rc<str>, String> {
  let Some(name) = leaf_value(node) else {
    return Err(format!("{declaration} name must be a token"));
  };
  if name.is_empty() || (require_dollar && (name == "$" || !name.starts_with('$'))) {
    return Err(format!("{declaration} expects a non-empty `$name`, got `{name}`"));
  }
  Ok(Rc::from(name))
}

fn parse_local_decl(node: &Cirru, collector: &mut LocalsCollector) -> Result<CalxLocalDecl, String> {
  let Cirru::List(items) = node else {
    return Err("local declaration must be an expression".to_string());
  };
  if items.len() != 3 {
    return Err(format!("local expects `$name TYPE`, got {items:?}"));
  }
  let name = parse_decl_name(&items[1], "local", true)?;
  let Some(value_type) = leaf_value(&items[2]) else {
    return Err("local type must be a token".to_string());
  };
  let value_type = value_type.parse::<CalxType>()?;
  collector.declare(&name)?;
  Ok(CalxLocalDecl::new(name, value_type))
}

fn parse_legacy_local_decl(node: &Cirru, collector: &mut LocalsCollector) -> Result<CalxLocalDecl, String> {
  let Cirru::List(items) = node else {
    return Err("local.new must be an expression".to_string());
  };
  if items.len() > 2 {
    return Err(format!("local.new accepts at most one `$name`, got {items:?}"));
  }
  let name = match items.get(1) {
    Some(node) => parse_decl_name(node, "local.new", true)?,
    None => Rc::from(format!("${}", collector.len())),
  };
  collector.declare(&name)?;
  Ok(CalxLocalDecl {
    name,
    value_type: CalxBoundaryType::Dynamic,
    span: None,
  })
}

fn parse_global_decl(nodes: &[Cirru], span: Option<SourceSpan>) -> Result<CalxGlobalDecl, String> {
  if nodes.len() != 4 {
    return Err(format!("global expects `$name (const|mut TYPE) INITIALIZER`, got {nodes:?}"));
  }
  let name = parse_decl_name(&nodes[1], "global", true)?;
  let Cirru::List(contract) = &nodes[2] else {
    return Err("global contract must be `(const|mut TYPE)`".to_string());
  };
  if contract.len() != 2 {
    return Err(format!("global contract expects mutability and type, got {contract:?}"));
  }
  let mutability = match leaf_value(&contract[0]) {
    Some("const") => CalxMutability::Const,
    Some("mut") => CalxMutability::Mut,
    Some(value) => return Err(format!("global mutability must be `const` or `mut`, got `{value}`")),
    None => return Err("global mutability must be a token".to_string()),
  };
  let Some(value_type) = leaf_value(&contract[1]) else {
    return Err("global type must be a token".to_string());
  };
  let value_type = value_type.parse::<CalxType>()?;
  let Some(initial) = leaf_value(&nodes[3]) else {
    return Err("global initializer must be a scalar literal".to_string());
  };
  let initial = initial.parse::<crate::Calx>()?;
  Ok(CalxGlobalDecl::new(name, value_type, mutability, initial).with_span(span))
}

fn parse_import_decl(nodes: &[Cirru], span: Option<SourceSpan>) -> Result<CalxImportDecl, String> {
  if nodes.len() != 3 {
    return Err(format!("import-fn expects `NAME (PARAMS... -> [RESULT])`, got {nodes:?}"));
  }
  let name = parse_decl_name(&nodes[1], "import-fn", false)?;
  let Cirru::List(signature) = &nodes[2] else {
    return Err("import-fn signature must be an expression".to_string());
  };
  if signature.iter().any(|node| matches!(node, Cirru::List(_))) {
    return Err("import-fn parameter and result types must be plain tokens".to_string());
  }
  let arrow_count = signature.iter().filter(|node| leaf_value(node) == Some("->")).count();
  if arrow_count != 1 {
    return Err(format!("import-fn signature requires exactly one `->`, found {arrow_count}"));
  }
  let (params, results) = parse_block_types(&nodes[2])?;
  if results.len() > 1 {
    return Err(format!("import-fn supports zero or one result, got {results:?}"));
  }
  Ok(CalxImportDecl::new(name, params, results.into_iter().next()).with_span(span))
}

fn parse_instr(
  ptr_base: usize,
  node: &Cirru,
  collector: &mut LocalsCollector,
  global_names: &HashMap<String, usize>,
) -> Result<Vec<CalxSyntax>, String> {
  match node {
    Cirru::Leaf(_) => Err(format!("expected expr of instruction, {node}")),
    Cirru::List(xs) => {
      if xs.is_empty() {
        return Err(String::from("empty expr"));
      }
      let i0 = &xs[0];

      match i0 {
        Cirru::List(_) => Err(format!("expected instruction name in a string, got {i0}")),
        Cirru::Leaf(name) => match &**name {
          "local.get" => {
            if xs.len() != 2 {
              return Err(format!("local.get expected a position, {xs:?}"));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalGet(idx)])
          }
          "local.set" => {
            if xs.len() != 2 {
              return Err(format!("local.set expected a position, {xs:?}"));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalSet(idx)])
          }
          "local.tee" => {
            if xs.len() != 2 {
              return Err(format!("local.tee expected a position, {xs:?}"));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalTee(idx)])
          }
          "local.new" => Err("local.new is only allowed in the function declaration prefix".to_string()),
          "global.get" => {
            if xs.len() != 2 {
              return Err(format!("global.get expected a position, {xs:?}"));
            }
            let idx = parse_global_idx(&xs[1], global_names)?;
            Ok(vec![CalxSyntax::GlobalGet(idx)])
          }
          "global.set" => {
            if xs.len() != 2 {
              return Err(format!("global.set expected a position, {xs:?}"));
            }
            let idx = parse_global_idx(&xs[1], global_names)?;
            Ok(vec![CalxSyntax::GlobalSet(idx)])
          }
          "global.new" => Ok(vec![CalxSyntax::GlobalNew]),
          "const" => {
            if xs.len() != 2 {
              return Err(format!("const takes exactly 1 argument, got {xs:?}"));
            }
            match &xs[1] {
              Cirru::Leaf(s) => {
                let p1 = s.parse()?;
                Ok(vec![CalxSyntax::Const(p1)])
              }
              Cirru::List(a) => Err(format!("`const` not supporting list here: {a:?}")),
            }
          }
          "dup" => Ok(vec![CalxSyntax::Dup]),
          "drop" => Ok(vec![CalxSyntax::Drop]),
          "i.add" => Ok(vec![CalxSyntax::IntAdd]),
          "i.mul" => Ok(vec![CalxSyntax::IntMul]),
          "i.div" => Ok(vec![CalxSyntax::IntDiv]),
          "i.neg" => Ok(vec![CalxSyntax::IntNeg]),
          "i.rem" => Ok(vec![CalxSyntax::IntRem]),
          "i.shr" => Ok(vec![CalxSyntax::IntShr]),
          "i.shl" => Ok(vec![CalxSyntax::IntShl]),
          "i.eq" => Ok(vec![CalxSyntax::IntEq]),
          "i.ne" => Ok(vec![CalxSyntax::IntNe]),
          "i.lt" => Ok(vec![CalxSyntax::IntLt]),
          "i.le" => Ok(vec![CalxSyntax::IntLe]),
          "i.gt" => Ok(vec![CalxSyntax::IntGt]),
          "i.ge" => Ok(vec![CalxSyntax::IntGe]),
          "f.eq" => Ok(vec![CalxSyntax::F64Eq]),
          "f.ne" => Ok(vec![CalxSyntax::F64Ne]),
          "f.lt" => Ok(vec![CalxSyntax::F64Lt]),
          "f.le" => Ok(vec![CalxSyntax::F64Le]),
          "f.gt" => Ok(vec![CalxSyntax::F64Gt]),
          "f.ge" => Ok(vec![CalxSyntax::F64Ge]),
          "f64-buffer.len" => Ok(vec![CalxSyntax::F64BufferLen]),
          "f64.to-i64-index" => Ok(vec![CalxSyntax::F64ToI64Index]),
          "f64-buffer.get" => Ok(vec![CalxSyntax::F64BufferGet]),
          "add" => Ok(vec![CalxSyntax::Add]),
          "mul" => Ok(vec![CalxSyntax::Mul]),
          "div" => Ok(vec![CalxSyntax::Div]),
          "neg" => Ok(vec![CalxSyntax::Neg]),
          "new-list" | "list.get" | "list.set" | "new-link" | "and" | "or" | "not" => {
            Err(format!("instruction `{name}` is reserved but not implemented"))
          }
          "br-if" => {
            if xs.len() != 2 {
              return Err(format!("br-if expected a position, {xs:?}"));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::BrIf(idx)])
          }
          "br" => {
            if xs.len() != 2 {
              return Err(format!("br expected a position, {xs:?}"));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::Br(idx)])
          }
          "block" => parse_block(ptr_base, xs, false, collector, global_names),
          "loop" => parse_block(ptr_base, xs, true, collector, global_names),
          "echo" => Ok(vec![CalxSyntax::Echo]),
          "call" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {xs:?}"));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::Call(Rc::from(name))])
          }
          "return-call" => {
            if xs.len() != 2 {
              return Err(format!("return-call expected function name, {xs:?}"));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::ReturnCall(Rc::from(name))])
          }
          "call-import" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {xs:?}"));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::CallImport(Rc::from(name))])
          }
          "unreachable" => Ok(vec![CalxSyntax::Unreachable]),
          "nop" => Ok(vec![CalxSyntax::Nop]),
          ";;" => {
            // commenOk
            Ok(vec![])
          }
          "quit" => {
            if xs.len() != 2 {
              return Err(format!("quit expected a position, {xs:?}"));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::Quit(idx)])
          }
          "return" => Ok(vec![CalxSyntax::Return]),

          "assert" => {
            if xs.len() != 2 {
              return Err(format!("assert expected an extra message, {xs:?}"));
            }
            let message: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("assert expected a message, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::Assert(Rc::from(message))])
          }
          "inspect" => Ok(vec![CalxSyntax::Inspect]),
          "if" => parse_if(ptr_base, xs, collector, global_names),
          _ => Err(format!("unknown instruction: {name} in {xs:?}")),
        },
      }
    }
  }
}

fn parse_local_idx(x: &Cirru, collector: &mut LocalsCollector) -> Result<usize, String> {
  match x {
    Cirru::Leaf(s) => match s.chars().next() {
      Some(c) => {
        if c == '$' {
          collector
            .resolve(s)
            .ok_or_else(|| format!("unknown local `{s}`; declare it before use"))
        } else {
          parse_usize(s)
        }
      }
      None => Err(String::from("invalid empty name")),
    },
    Cirru::List(_) => Err(format!("expected token, got {x}")),
  }
}

fn parse_global_idx(x: &Cirru, global_names: &HashMap<String, usize>) -> Result<usize, String> {
  match x {
    Cirru::Leaf(name) if name.starts_with('$') => global_names
      .get(name.as_ref())
      .copied()
      .ok_or_else(|| format!("unknown global `{name}`")),
    Cirru::Leaf(index) => parse_usize(index),
    Cirru::List(_) => Err(format!("expected global name or index, got {x}")),
  }
}

pub fn parse_usize(s: &str) -> Result<usize, String> {
  match s.parse::<usize>() {
    Ok(u) => Ok(u),
    Err(e) => Err(format!("failed to parse: {e}")),
  }
}

fn parse_block(
  ptr_base: usize,
  xs: &[Cirru],
  looped: bool,
  collector: &mut LocalsCollector,
  global_names: &HashMap<String, usize>,
) -> Result<Vec<CalxSyntax>, String> {
  if xs.len() < 2 {
    return Err(format!(
      "{} expected a type signature, got {xs:?}",
      if looped { "loop" } else { "block" }
    ));
  }

  let mut p = ptr_base + 1;
  let mut chunk: Vec<CalxSyntax> = vec![];
  let (params_types, ret_types) = parse_block_types(&xs[1])?;
  for (idx, line) in xs.iter().enumerate() {
    if idx > 1 {
      let lines = extract_nested(line)?;
      for expanded in &lines {
        let instrs = parse_instr(p, expanded, collector, global_names)?;
        for y in instrs {
          p += 1;
          chunk.push(y);
        }
      }
    }
  }
  chunk.push(CalxSyntax::BlockEnd(looped));

  chunk.insert(
    0,
    CalxSyntax::Block {
      looped,
      from: ptr_base + 1,
      to: p,
      params_types: Rc::new(params_types),
      ret_types: Rc::new(ret_types),
    },
  );
  Ok(chunk)
}

fn parse_if(
  ptr_base: usize,
  xs: &[Cirru],
  collector: &mut LocalsCollector,
  global_names: &HashMap<String, usize>,
) -> Result<Vec<CalxSyntax>, String> {
  if xs.len() != 4 && xs.len() != 3 {
    return Err(format!("if expected 2 or 3 arguments, got {xs:?}"));
  }
  let types = parse_block_types(&xs[1])?;
  let ret_types = types.1;
  let then_syntax = parse_do(&xs[2], collector, global_names)?;
  let else_syntax = if xs.len() == 4 {
    parse_do(&xs[3], collector, global_names)?
  } else {
    vec![]
  };

  let mut p = ptr_base + 1; // leave a place for if instruction
  let mut chunk: Vec<CalxSyntax> = vec![];

  // put else branch first, and use jmp to target then branch
  for instr in else_syntax {
    p += 1;
    chunk.push(instr);
  }
  p += 1;
  let else_at = p;
  chunk.push(CalxSyntax::ElseEnd);
  for instr in then_syntax {
    p += 1;
    chunk.push(instr);
  }

  p += 1;
  chunk.push(CalxSyntax::ThenEnd);

  let to = p;

  chunk.insert(
    0,
    CalxSyntax::If {
      ret_types: Rc::new(ret_types),
      else_at,
      to,
    },
  );

  Ok(chunk)
}

fn parse_do(xs: &Cirru, collector: &mut LocalsCollector, global_names: &HashMap<String, usize>) -> Result<Vec<CalxSyntax>, String> {
  match xs {
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {xs}")),
    Cirru::List(ys) => {
      let Some(x0) = ys.first() else {
        return Err("expected `do`, got an empty expression".to_string());
      };
      if !leaf_is(x0, "do") {
        return Err(format!("expected do, got {x0}"));
      }

      let mut chunk: Vec<CalxSyntax> = vec![];
      for (idx, x) in ys.iter().enumerate() {
        if idx > 0 {
          let lines = extract_nested(x)?;
          for expanded in &lines {
            let instrs = parse_instr(idx, expanded, collector, global_names)?;
            for y in instrs {
              chunk.push(y);
            }
          }
        }
      }
      Ok(chunk)
    }
  }
}

/// parameters might be named, need to check, by default use integers
pub fn parse_fn_types(xs: &Cirru, collector: &mut LocalsCollector) -> Result<(Vec<CalxType>, Vec<CalxType>), String> {
  match xs {
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {xs}")),
    Cirru::List(ys) => {
      let mut params: Vec<CalxType> = vec![];
      let mut returns: Vec<CalxType> = vec![];
      let mut ret_mode = false;

      for x in ys {
        match x {
          Cirru::Leaf(t) => {
            if &**t == "->" {
              ret_mode = true;
            } else {
              let ty = t.parse()?;
              if ret_mode {
                returns.push(ty);
              } else {
                // track names in collector, if NOT named, use a string of index
                let name = format!("${}", params.len());
                collector.declare(&name)?;
                params.push(ty);
              }
            }
          }

          Cirru::List(zs) => {
            if ret_mode {
              return Err(format!("invalid syntax, return values should not have names, got {x:?}"));
            }
            if zs.len() != 2 {
              return Err(format!("invalid syntax, expected name and type, got {x:?}"));
            }
            let name_str = match &zs[0] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("invalid syntax, expected name, got {x:?}")),
            };
            let ty = match &zs[1] {
              Cirru::Leaf(s) => s.parse()?,
              Cirru::List(_) => return Err(format!("invalid syntax, expected type, got {x:?}")),
            };
            collector.declare(&name_str)?;
            params.push(ty);
          }
        }
      }

      Ok((params, returns))
    }
  }
}

/// does not need names in block
pub fn parse_block_types(xs: &Cirru) -> Result<(Vec<CalxType>, Vec<CalxType>), String> {
  match xs {
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {xs}")),
    Cirru::List(ys) => {
      let mut params: Vec<CalxType> = vec![];
      let mut returns: Vec<CalxType> = vec![];
      let mut ret_mode = false;

      for x in ys {
        if let Cirru::Leaf(t) = x {
          if &**t == "->" {
            ret_mode = true;
          } else {
            let ty = t.parse()?;
            if ret_mode {
              returns.push(ty);
            } else {
              params.push(ty);
            }
          }
        }
      }

      Ok((params, returns))
    }
  }
}

/// rather stupid function to extract nested calls before current call
/// TODO better have some tests
pub fn extract_nested(xs: &Cirru) -> Result<Vec<Cirru>, String> {
  match xs {
    Cirru::Leaf(x) => Err(format!("not extracting leaf: {x}")),
    Cirru::List(ys) => match ys.first() {
      None => Err(String::from("unexpected empty expression")),
      Some(Cirru::List(zs)) => Err(format!("unexpected nested instruction name: {zs:?}")),
      Some(Cirru::Leaf(zs)) => match &**zs {
        "block" | "loop" | "if" | "do" => Ok(vec![xs.to_owned()]),
        _ => {
          let mut pre: Vec<Cirru> = vec![];
          let mut chunk: Vec<Cirru> = vec![Cirru::Leaf(zs.to_owned())];
          for (idx, y) in ys.iter().enumerate() {
            if idx > 0 {
              match y {
                Cirru::Leaf(_) => chunk.push(y.to_owned()),
                Cirru::List(_) => {
                  for e in extract_nested(y)? {
                    pre.push(e);
                  }
                }
              }
            }
          }
          pre.push(Cirru::List(chunk));
          Ok(pre)
        }
      },
    },
  }
}

pub fn leaf_is(x: &Cirru, name: &str) -> bool {
  if let Cirru::Leaf(y) = x {
    if &**y == name {
      return true;
    }
  }
  false
}

#[derive(Debug)]
struct SourceToken {
  value: String,
  span: SourceSpan,
  structural: bool,
}

#[derive(Debug)]
struct LocatedCirru<'a> {
  node: &'a Cirru,
  span: Option<SourceSpan>,
  children: Vec<LocatedCirru<'a>>,
}

impl LocatedCirru<'_> {
  fn head(&self) -> Option<&str> {
    match self.children.first().map(|child| child.node) {
      Some(Cirru::Leaf(value)) => Some(value),
      Some(Cirru::List(_)) | None => None,
    }
  }

  fn head_span(&self) -> Option<SourceSpan> {
    self.children.first().and_then(|child| child.span.clone())
  }
}

fn cirru_parse_error(source: Rc<str>, error: CirruError) -> ParseError {
  let span = error.context.as_ref().map(|context| {
    let position = SourcePosition::new(context.pos.line, context.pos.column, context.pos.offset);
    Box::new(SourceSpan::new(source, position, position))
  });
  ParseError {
    code: DiagnosticCode::CirruParse,
    message: error.kind.to_string(),
    function: None,
    instruction_index: None,
    span,
  }
}

fn locate_node<'a>(node: &'a Cirru, tokens: &[SourceToken], cursor: &mut usize) -> Result<LocatedCirru<'a>, ParseError> {
  match node {
    Cirru::Leaf(value) => {
      while tokens.get(*cursor).is_some_and(|token| token.structural) {
        *cursor += 1;
      }
      let Some(token) = tokens.get(*cursor) else {
        return Err(ParseError {
          code: DiagnosticCode::InstructionParse,
          message: format!("could not locate parsed token `{value}` in source"),
          function: None,
          instruction_index: None,
          span: None,
        });
      };
      if token.value.as_str() != value.as_ref() {
        return Err(ParseError {
          code: DiagnosticCode::InstructionParse,
          message: format!("source token `{}` did not match parsed token `{value}`", token.value),
          function: None,
          instruction_index: None,
          span: Some(Box::new(token.span.clone())),
        });
      }
      *cursor += 1;
      Ok(LocatedCirru {
        node,
        span: Some(token.span.clone()),
        children: vec![],
      })
    }
    Cirru::List(items) => {
      let children = items
        .iter()
        .map(|item| locate_node(item, tokens, cursor))
        .collect::<Result<Vec<_>, _>>()?;
      let span = children
        .iter()
        .find_map(|child| child.span.as_ref())
        .zip(children.iter().rev().find_map(|child| child.span.as_ref()))
        .map(|(first, last)| SourceSpan::new(first.source.clone(), first.start, last.end));
      Ok(LocatedCirru { node, span, children })
    }
  }
}

fn function_source_spans(function: &LocatedCirru<'_>) -> Vec<Option<SourceSpan>> {
  let mut spans = vec![];
  for line in function.children.iter().skip(3) {
    match line.head() {
      Some("local") => {}
      Some("local.new") => spans.push(line.head_span()),
      _ => append_expanded_spans(line, &mut spans),
    }
  }
  spans
}

fn attach_local_source_spans(function: &mut CalxFunc, located: &LocatedCirru<'_>) {
  let declarations = Rc::make_mut(&mut function.locals);
  let mut index = 0;
  for line in located.children.iter().skip(3) {
    if matches!(line.head(), Some("local" | "local.new")) {
      if let Some(declaration) = declarations.get_mut(index) {
        declaration.span = line.span.clone();
      }
      index += 1;
    }
  }
}

fn locate_function_parse_failure(
  nodes: &[Cirru],
  function: &LocatedCirru<'_>,
  global_names: &HashMap<String, usize>,
) -> (Option<usize>, Option<SourceSpan>) {
  if nodes.len() <= 3 || (!leaf_is(&nodes[0], "fn") && !leaf_is(&nodes[0], "defn")) {
    return (None, function.span.clone());
  }
  if !matches!(nodes[1], Cirru::Leaf(_)) {
    return (None, function.children.get(1).and_then(|node| node.span.clone()));
  }

  let mut collector = LocalsCollector::new();
  if parse_fn_types(&nodes[2], &mut collector).is_err() {
    return (None, function.children.get(2).and_then(|node| node.span.clone()));
  }

  let mut instruction_index = 0;
  let mut executable_started = false;
  for line in function.children.iter().skip(3) {
    match line.head() {
      Some("local") if !executable_started => {
        if parse_local_decl(line.node, &mut collector).is_err() {
          return (None, line.span.clone());
        }
        continue;
      }
      Some("local.new") if !executable_started => {
        if parse_legacy_local_decl(line.node, &mut collector).is_err() {
          return (Some(instruction_index), line.span.clone());
        }
        instruction_index += 1;
        continue;
      }
      Some("local" | "local.new") => return (Some(instruction_index), line.span.clone()),
      Some(";;") if !executable_started => continue,
      _ => executable_started = true,
    }
    let Ok(expanded_nodes) = extract_nested_located(line) else {
      return (Some(instruction_index), line.head_span().or_else(|| line.span.clone()));
    };
    for (expanded, span) in expanded_nodes {
      match parse_instr(instruction_index, &expanded, &mut collector, global_names) {
        Ok(syntax) => instruction_index += syntax.len(),
        Err(_) => return (Some(instruction_index), span),
      }
    }
  }

  (None, function.span.clone())
}

fn extract_nested_located(node: &LocatedCirru<'_>) -> Result<Vec<(Cirru, Option<SourceSpan>)>, String> {
  let Cirru::List(items) = node.node else {
    return Err(format!("not extracting leaf: {}", node.node));
  };
  let Some(first) = items.first() else {
    return Err("unexpected empty expression".to_string());
  };
  let Cirru::Leaf(name) = first else {
    return Err(format!("unexpected nested instruction name: {first:?}"));
  };

  if matches!(&**name, "block" | "loop" | "if" | "do") {
    return Ok(vec![(node.node.clone(), node.head_span())]);
  }

  let mut expanded = vec![];
  let mut current = vec![Cirru::Leaf(name.clone())];
  for child in node.children.iter().skip(1) {
    match child.node {
      Cirru::Leaf(_) => current.push(child.node.clone()),
      Cirru::List(_) => expanded.extend(extract_nested_located(child)?),
    }
  }
  expanded.push((Cirru::List(current), node.head_span()));
  Ok(expanded)
}

fn append_expanded_spans(node: &LocatedCirru<'_>, spans: &mut Vec<Option<SourceSpan>>) {
  match node.head() {
    Some("block" | "loop") => append_block_spans(node, spans),
    Some("if") => append_if_spans(node, spans),
    Some("do") => append_do_spans(node, spans),
    Some(";;") => {}
    Some(_) => {
      for child in node.children.iter().skip(1) {
        if matches!(child.node, Cirru::List(_)) {
          append_expanded_spans(child, spans);
        }
      }
      spans.push(node.head_span());
    }
    None => {}
  }
}

fn append_block_spans(node: &LocatedCirru<'_>, spans: &mut Vec<Option<SourceSpan>>) {
  let control_span = node.head_span();
  spans.push(control_span.clone());
  for line in node.children.iter().skip(2) {
    append_expanded_spans(line, spans);
  }
  spans.push(control_span);
}

fn append_if_spans(node: &LocatedCirru<'_>, spans: &mut Vec<Option<SourceSpan>>) {
  let control_span = node.head_span();
  spans.push(control_span.clone());

  if let Some(else_branch) = node.children.get(3) {
    append_do_spans(else_branch, spans);
    spans.push(else_branch.head_span().or_else(|| control_span.clone()));
  } else {
    spans.push(control_span.clone());
  }

  if let Some(then_branch) = node.children.get(2) {
    append_do_spans(then_branch, spans);
    spans.push(then_branch.head_span().or(control_span));
  } else {
    spans.push(control_span);
  }
}

fn append_do_spans(node: &LocatedCirru<'_>, spans: &mut Vec<Option<SourceSpan>>) {
  for line in node.children.iter().skip(1) {
    append_expanded_spans(line, spans);
  }
}

fn scan_source_tokens(source: Rc<str>, code: &str) -> Vec<SourceToken> {
  let mut tokens = vec![];
  let mut offset = 0;
  let mut line = 1;
  let mut column = 1;

  while offset < code.len() {
    let c = code[offset..].chars().next().expect("offset remains on a character boundary");
    if matches!(c, ' ' | '\n' | '(' | ')') {
      advance_position(c, &mut offset, &mut line, &mut column);
      continue;
    }

    let start = SourcePosition::new(line, column, offset);
    let mut value = String::new();
    let quoted = c == '"';
    if quoted {
      advance_position(c, &mut offset, &mut line, &mut column);
      while offset < code.len() {
        let next = code[offset..].chars().next().expect("offset remains on a character boundary");
        advance_position(next, &mut offset, &mut line, &mut column);
        match next {
          '"' => break,
          '\\' if offset < code.len() => {
            let escaped = code[offset..].chars().next().expect("offset remains on a character boundary");
            advance_position(escaped, &mut offset, &mut line, &mut column);
            match escaped {
              '"' => value.push('"'),
              '\'' => value.push('\''),
              't' => value.push('\t'),
              'n' => value.push('\n'),
              'r' => value.push('\r'),
              '\\' => value.push('\\'),
              other => value.push(other),
            }
          }
          other => value.push(other),
        }
      }
    } else {
      while offset < code.len() {
        let next = code[offset..].chars().next().expect("offset remains on a character boundary");
        if matches!(next, ' ' | '\n' | '(' | ')' | '"') {
          break;
        }
        value.push(next);
        advance_position(next, &mut offset, &mut line, &mut column);
      }
    }

    let end = SourcePosition::new(line, column, offset);
    let structural = !quoted && matches!(value.as_str(), "$" | ",");
    tokens.push(SourceToken {
      value,
      span: SourceSpan::new(source.clone(), start, end),
      structural,
    });
  }

  tokens
}

fn advance_position(c: char, offset: &mut usize, line: &mut usize, column: &mut usize) {
  *offset += c.len_utf8();
  if c == '\n' {
    *line += 1;
    *column = 1;
  } else {
    *column += 1;
  }
}
