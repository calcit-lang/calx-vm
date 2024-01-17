/*! Parse Cirru into Calx instructions
 *
 */

mod locals;

use std::rc::Rc;

use lazy_static::lazy_static;
use regex::Regex;

use cirru_parser::Cirru;

use crate::primes::{Calx, CalxFunc, CalxInstr, CalxType};

use self::locals::LocalsCollector;

/// parses
/// ```cirru
/// fn <f-name> (i64 f64)
///   const 1
///   echo
/// ```
pub fn parse_function(nodes: &[Cirru]) -> Result<CalxFunc, String> {
  if nodes.len() <= 3 {
    return Err(String::from("Not a function"));
  }

  if let Cirru::Leaf(x) = nodes[0].to_owned() {
    if &*x == "fn" {
      // ok
    } else {
      return Err(String::from("invalid"));
    }
  } else {
    return Err(String::from("invalid"));
  }

  let name: Box<str> = if let Cirru::Leaf(x) = nodes[1].to_owned() {
    x
  } else {
    return Err(String::from("invalid name"));
  };

  let mut body: Vec<CalxInstr> = vec![];
  let mut locals_collector: LocalsCollector = LocalsCollector::new();

  let (params_types, ret_types) = parse_fn_types(&nodes[2], &mut locals_collector)?;

  let mut ptr_base: usize = 0;
  for (idx, line) in nodes.iter().enumerate() {
    if idx >= 3 {
      for expanded in extract_nested(line)? {
        // println!("expanded {}", expanded);
        let instrs = parse_instr(ptr_base, &expanded, &mut locals_collector)?;

        for instr in instrs {
          ptr_base += 1;
          body.push(instr);
        }
      }
    }
  }

  Ok(CalxFunc {
    name: Rc::new(name.to_string()),
    params_types: params_types.into(),
    ret_types: Rc::new(ret_types),
    local_names: Rc::new(locals_collector.locals),
    instrs: Rc::new(body),
  })
}

pub fn parse_instr(ptr_base: usize, node: &Cirru, collector: &mut LocalsCollector) -> Result<Vec<CalxInstr>, String> {
  match node {
    Cirru::Leaf(_) => Err(format!("expected expr of instruction, {}", node)),
    Cirru::List(xs) => {
      if xs.is_empty() {
        return Err(String::from("empty expr"));
      }
      let i0 = xs[0].to_owned();

      match i0 {
        Cirru::List(_) => Err(format!("expected instruction name in a string, got {}", i0)),
        Cirru::Leaf(name) => match &*name {
          "local.get" => {
            if xs.len() != 2 {
              return Err(format!("local.get expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxInstr::LocalGet(idx)])
          }
          "local.set" => {
            if xs.len() != 2 {
              return Err(format!("local.set expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxInstr::LocalSet(idx)])
          }
          "local.tee" => {
            if xs.len() != 2 {
              return Err(format!("list.tee expected a position, {:?}", xs));
            }
            let idx: usize = parse_local_idx(&xs[1], collector)?;
            Ok(vec![CalxInstr::LocalSet(idx)])
          }
          "local.new" => Ok(vec![CalxInstr::LocalNew]),
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
            Ok(vec![CalxInstr::GlobalGet(idx)])
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
            Ok(vec![CalxInstr::GlobalSet(idx)])
          }
          "global.new" => Ok(vec![CalxInstr::GlobalNew]),
          "const" => {
            if xs.len() != 2 {
              return Err(format!("const takes exactly 1 argument, got {:?}", xs));
            }
            match &xs[1] {
              Cirru::Leaf(s) => {
                let p1 = parse_value(s)?;
                Ok(vec![CalxInstr::Const(p1)])
              }
              Cirru::List(a) => Err(format!("`const` not supporting list here: {:?}", a)),
            }
          }
          "dup" => Ok(vec![CalxInstr::Dup]),
          "drop" => Ok(vec![CalxInstr::Drop]),
          "i.add" => Ok(vec![CalxInstr::IntAdd]),
          "i.mul" => Ok(vec![CalxInstr::IntMul]),
          "i.div" => Ok(vec![CalxInstr::IntDiv]),
          "i.neg" => Ok(vec![CalxInstr::IntNeg]),
          "i.rem" => Ok(vec![CalxInstr::IntRem]),
          "i.shr" => Ok(vec![CalxInstr::IntShr]),
          "i.shl" => Ok(vec![CalxInstr::IntShl]),
          "i.eq" => Ok(vec![CalxInstr::IntEq]),
          "i.ne" => Ok(vec![CalxInstr::IntNe]),
          "i.lt" => Ok(vec![CalxInstr::IntLt]),
          "i.le" => Ok(vec![CalxInstr::IntLe]),
          "i.gt" => Ok(vec![CalxInstr::IntGt]),
          "i.ge" => Ok(vec![CalxInstr::IntGe]),
          "add" => Ok(vec![CalxInstr::Add]),
          "mul" => Ok(vec![CalxInstr::Mul]),
          "div" => Ok(vec![CalxInstr::Div]),
          "neg" => Ok(vec![CalxInstr::Neg]),
          "new-list" => Ok(vec![CalxInstr::NewList]),
          "list.get" => Ok(vec![CalxInstr::ListGet]),
          "list.set" => Ok(vec![CalxInstr::ListSet]),
          "new-link" => Ok(vec![CalxInstr::NewLink]),
          // TODO
          "and" => Ok(vec![CalxInstr::And]),
          "or" => Ok(vec![CalxInstr::Or]),
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
            Ok(vec![CalxInstr::BrIf(idx)])
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
            Ok(vec![CalxInstr::Br(idx)])
          }
          "block" => parse_block(ptr_base, xs, false, collector),
          "loop" => parse_block(ptr_base, xs, true, collector),
          "echo" => Ok(vec![CalxInstr::Echo]),
          "call" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxInstr::Call((*name).to_owned())])
          }
          "return-call" => {
            if xs.len() != 2 {
              return Err(format!("return-call expected function name, {:?}", xs));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxInstr::ReturnCall((*name).to_owned())])
          }
          "call-import" => {
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
            }
            let name: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            };

            Ok(vec![CalxInstr::CallImport((*name).to_owned())])
          }
          "unreachable" => Ok(vec![CalxInstr::Unreachable]),
          "nop" => Ok(vec![CalxInstr::Nop]),
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
            Ok(vec![CalxInstr::Quit(idx)])
          }
          "return" => Ok(vec![CalxInstr::Return]),

          "assert" => {
            if xs.len() != 2 {
              return Err(format!("assert expected an extra message, {:?}", xs));
            }
            let message: Box<str> = match &xs[1] {
              Cirru::Leaf(s) => s.to_owned(),
              Cirru::List(_) => return Err(format!("assert expected a message, got {:?}", xs[1])),
            };

            Ok(vec![CalxInstr::Assert((*message).to_owned())])
          }
          "inspect" => Ok(vec![CalxInstr::Inspect]),
          _ => Err(format!("unknown instruction: {}", name)),
        },
      }
    }
  }
}

lazy_static! {
  static ref FLOAT_PATTERN: Regex = Regex::new("^-?\\d+\\.(\\d+)?$").unwrap();
  static ref INT_PATTERN: Regex = Regex::new("^-?\\d+$").unwrap();
  static ref USIZE_PATTERN: Regex = Regex::new("^\\d+$").unwrap();
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

pub fn parse_value(s: &str) -> Result<Calx, String> {
  match s {
    "nil" => Ok(Calx::Nil),
    "true" => Ok(Calx::Bool(true)),
    "false" => Ok(Calx::Bool(false)),
    "" => Err(String::from("unknown empty string")),
    _ => {
      let s0 = s.chars().next().unwrap();
      if s0 == '|' || s0 == ':' {
        Ok(Calx::Str(s[1..s.len()].to_owned()))
      } else if FLOAT_PATTERN.is_match(s) {
        match s.parse::<f64>() {
          Ok(u) => Ok(Calx::F64(u)),
          Err(e) => Err(format!("failed to parse: {}", e)),
        }
      } else if INT_PATTERN.is_match(s) {
        match s.parse::<i64>() {
          Ok(u) => Ok(Calx::I64(u)),
          Err(e) => Err(format!("failed to parse: {}", e)),
        }
      } else {
        Err(format!("unknown value: {}", s))
      }
    }
  }
}

pub fn parse_usize(s: &str) -> Result<usize, String> {
  match s.parse::<usize>() {
    Ok(u) => Ok(u),
    Err(e) => Err(format!("failed to parse: {}", e)),
  }
}

pub fn parse_block(ptr_base: usize, xs: &[Cirru], looped: bool, collector: &mut LocalsCollector) -> Result<Vec<CalxInstr>, String> {
  let mut p = ptr_base + 1;
  let mut chunk: Vec<CalxInstr> = vec![];
  let (params_types, ret_types) = parse_block_types(&xs[1])?;
  for (idx, line) in xs.iter().enumerate() {
    if idx > 1 {
      let instrs = parse_instr(p, line, collector)?;
      for y in instrs {
        p += 1;
        chunk.push(y);
      }
    }
  }
  chunk.push(CalxInstr::BlockEnd(looped));

  if looped && !ret_types.is_empty() {
    println!("return types for loop actuall not checked: {:?}", ret_types);
  }

  chunk.insert(
    0,
    CalxInstr::Block {
      looped,
      from: ptr_base + 1,
      to: p,
      params_types: Rc::new(params_types),
      ret_types: Rc::new(ret_types),
    },
  );
  Ok(chunk)
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
              let ty = parse_type_name(t)?;
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
              Cirru::Leaf(s) => parse_type_name(s)?,
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
            let ty = parse_type_name(t)?;
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

fn parse_type_name(x: &str) -> Result<CalxType, String> {
  match x {
    "nil" => Ok(CalxType::Nil),
    "bool" => Ok(CalxType::Bool),
    "i64" => Ok(CalxType::I64),
    "f64" => Ok(CalxType::F64),
    "list" => Ok(CalxType::List),
    "link" => Ok(CalxType::Link),
    a => Err(format!("Unknown type: {}", a)),
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
        "block" | "loop" => {
          let mut chunk: Vec<Cirru> = vec![Cirru::Leaf(zs.to_owned())];
          for (idx, y) in ys.iter().enumerate() {
            if idx > 0 {
              for e in extract_nested(y)? {
                chunk.push(e);
              }
            }
          }
          Ok(vec![Cirru::List(chunk)])
        }
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
