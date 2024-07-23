/*! Parse Cirru into Calx instructions
 *
 */

mod locals;

use std::rc::Rc;

use cirru_parser::Cirru;

use crate::calx::CalxType;
use crate::syntax::CalxSyntax;
use crate::vm::func::CalxFunc;

use self::locals::LocalsCollector;

/// parses
/// ```cirru
/// fn <f-name> (i64 f64)
///   const 1
///   echo
/// ```
pub fn parse_function(nodes: &[Cirru]) -> Result<CalxFunc, String> {
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
  let mut locals_collector: LocalsCollector = LocalsCollector::new();

  let (params_types, ret_types) = parse_fn_types(&nodes[2], &mut locals_collector)?;

  let mut ptr_base: usize = 0;
  for (idx, line) in nodes.iter().enumerate() {
    if idx >= 3 {
      for expanded in extract_nested(line)? {
        // println!("expanded {}", expanded);
        let syntax = parse_instr(ptr_base, &expanded, &mut locals_collector)?;

        for instr in syntax {
          ptr_base += 1;
          body.push(instr);
        }
      }
    }
  }

  Ok(CalxFunc {
    name,
    params_types: params_types.into(),
    ret_types: Rc::new(ret_types),
    local_names: Rc::new(locals_collector.locals),
    syntax: Rc::new(body),
    instrs: Rc::new(vec![]),
  })
}

pub fn parse_instr(ptr_base: usize, node: &Cirru, collector: &mut LocalsCollector) -> Result<Vec<CalxSyntax>, String> {
  match node {
    Cirru::Leaf(_) => Err(format!("expected expr of instruction, {}", node)),
    Cirru::List(xs) => {
      if xs.is_empty() {
        return Err(String::from("empty expr"));
      }
      let i0 = &xs[0];

      match i0 {
        Cirru::List(_) => Err(format!("expected instruction name in a string, got {}", i0)),
        Cirru::Leaf(name) => match &**name {
          "local.get" => {
            if xs.len() != 2 {
              return Err(format!("local.get expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalGet(idx)])
          }
          "local.set" => {
            if xs.len() != 2 {
              return Err(format!("local.set expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalSet(idx)])
          }
          "local.tee" => {
            if xs.len() != 2 {
              return Err(format!("list.tee expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxSyntax::LocalSet(idx)])
          }
          "local.new" => Ok(vec![CalxSyntax::LocalNew]),
          "global.get" => {
            if xs.len() != 2 {
              return Err(format!("global.get expected a position, {:?}", xs));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::GlobalGet(idx)])
          }
          "global.set" => {
            if xs.len() != 2 {
              return Err(format!("global.set expected a position, {:?}", xs));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::GlobalSet(idx)])
          }
          "global.new" => Ok(vec![CalxSyntax::GlobalNew]),
          "const" => {
            if xs.len() != 2 {
              return Err(format!("const takes exactly 1 argument, got {:?}", xs));
            }
            match &xs[1] {
              Cirru::Leaf(s) => {
                let p1 = s.parse()?;
                Ok(vec![CalxSyntax::Const(p1)])
              }
              Cirru::List(a) => Err(format!("`const` not supporting list here: {:?}", a)),
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
          "add" => Ok(vec![CalxSyntax::Add]),
          "mul" => Ok(vec![CalxSyntax::Mul]),
          "div" => Ok(vec![CalxSyntax::Div]),
          "neg" => Ok(vec![CalxSyntax::Neg]),
          "new-list" => Ok(vec![CalxSyntax::NewList]),
          "list.get" => Ok(vec![CalxSyntax::ListGet]),
          "list.set" => Ok(vec![CalxSyntax::ListSet]),
          "new-link" => Ok(vec![CalxSyntax::NewLink]),
          // TODO
          "and" => Ok(vec![CalxSyntax::And]),
          "or" => Ok(vec![CalxSyntax::Or]),
          "br-if" => {
            if xs.len() != 2 {
              return Err(format!("br-if expected a position, {:?}", xs));
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
              return Err(format!("br expected a position, {:?}", xs));
            }
            let idx: usize = match &xs[1] {
              Cirru::Leaf(s) => parse_usize(s)?,
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            };
            Ok(vec![CalxSyntax::Br(idx)])
          }
          "block" => parse_block(ptr_base, xs, false, collector),
          "loop" => parse_block(ptr_base, xs, true, collector),
          "echo" => Ok(vec![CalxSyntax::Echo]),
          "call" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::Call(Rc::from(name))])
          }
          "return-call" => {
            if xs.len() != 2 {
              return Err(format!("return-call expected function name, {:?}", xs));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::ReturnCall(Rc::from(name))])
          }
          "call-import" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
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
              return Err(format!("quit expected a position, {:?}", xs));
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
              return Err(format!("assert expected an extra message, {:?}", xs));
            }
            let message: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => (**s).into(),
              Cirru::List(_) => return Err(format!("assert expected a message, got {:?}", xs[1])),
            };

            Ok(vec![CalxSyntax::Assert(Rc::from(message))])
          }
          "inspect" => Ok(vec![CalxSyntax::Inspect]),
          "if" => parse_if(ptr_base, xs, collector),
          _ => Err(format!("unknown instruction: {} in {:?}", name, xs)),
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
          Ok(collector.track(s))
        } else {
          parse_usize(s)
        }
      }
      None => Err(String::from("invalid empty name")),
    },
    Cirru::List(_) => Err(format!("expected token, got {}", x)),
  }
}

pub fn parse_usize(s: &str) -> Result<usize, String> {
  match s.parse::<usize>() {
    Ok(u) => Ok(u),
    Err(e) => Err(format!("failed to parse: {}", e)),
  }
}

pub fn parse_block(ptr_base: usize, xs: &[Cirru], looped: bool, collector: &mut LocalsCollector) -> Result<Vec<CalxSyntax>, String> {
  let mut p = ptr_base + 1;
  let mut chunk: Vec<CalxSyntax> = vec![];
  let (params_types, ret_types) = parse_block_types(&xs[1])?;
  for (idx, line) in xs.iter().enumerate() {
    if idx > 1 {
      let lines = extract_nested(line)?;
      for expanded in &lines {
        let instrs = parse_instr(p, expanded, collector)?;
        for y in instrs {
          p += 1;
          chunk.push(y);
        }
      }
    }
  }
  chunk.push(CalxSyntax::BlockEnd(looped));

  if looped && !ret_types.is_empty() {
    println!("return types for loop actuall not checked: {:?}", ret_types);
  }

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

pub fn parse_if(ptr_base: usize, xs: &[Cirru], collector: &mut LocalsCollector) -> Result<Vec<CalxSyntax>, String> {
  if xs.len() != 4 && xs.len() != 3 {
    return Err(format!("if expected 2 or 3 arguments, got {:?}", xs));
  }
  let types = parse_block_types(&xs[1])?;
  let ret_types = types.1;
  let then_syntax = parse_do(&xs[2], collector)?;
  let else_syntax = if xs.len() == 4 { parse_do(&xs[3], collector)? } else { vec![] };

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

pub fn parse_do(xs: &Cirru, collector: &mut LocalsCollector) -> Result<Vec<CalxSyntax>, String> {
  match xs {
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {}", xs)),
    Cirru::List(ys) => {
      let x0 = &ys[0];
      if !leaf_is(x0, "do") {
        return Err(format!("expected do, got {}", x0));
      }

      let mut chunk: Vec<CalxSyntax> = vec![];
      for (idx, x) in ys.iter().enumerate() {
        if idx > 0 {
          let lines = extract_nested(x)?;
          for expanded in &lines {
            let instrs = parse_instr(idx, expanded, collector)?;
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
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {}", xs)),
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
                collector.track(&name);
                params.push(ty);
              }
            }
          }

          Cirru::List(zs) => {
            if ret_mode {
              return Err(format!("invalid syntax, return values should not have names, got {:?}", x));
            }
            if zs.len() != 2 {
              return Err(format!("invalid syntax, expected name and type, got {:?}", x));
            }
            let name_str = match &zs[0] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("invalid syntax, expected name, got {:?}", x)),
            };
            let ty = match &zs[1] {
              Cirru::Leaf(s) => s.parse()?,
              Cirru::List(_) => return Err(format!("invalid syntax, expected type, got {:?}", x)),
            };
            collector.track(&name_str);
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
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {}", xs)),
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
    Cirru::Leaf(x) => Err(format!("not extracting leaf: {}", x)),
    Cirru::List(ys) => match ys.first() {
      None => Err(String::from("unexpected empty expression")),
      Some(Cirru::List(zs)) => Err(format!("unexpected nested instruction name: {:?}", zs)),
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
