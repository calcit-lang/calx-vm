/*! Parser Cirru into Calx instructions
 *
 */

use lazy_static::lazy_static;
use regex::Regex;

use cirru_parser::Cirru;

use crate::primes::{Calx, CalxFunc, CalxInstr, CalxType};

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
    if x == "fn" {
      // ok
    } else {
      return Err(String::from("invalid"));
    }
  } else {
    return Err(String::from("invalid"));
  }

  let name: String;
  if let Cirru::Leaf(x) = nodes[1].to_owned() {
    name = x;
  } else {
    return Err(String::from("invalid name"));
  }

  let (params_types, ret_types) = parse_types(&nodes[2])?;

  let mut body: Vec<CalxInstr> = vec![];
  let mut ptr_base: usize = 0;
  for (idx, line) in nodes.iter().enumerate() {
    if idx >= 3 {
      let instrs = parse_instr(ptr_base, line)?;

      for instr in instrs {
        ptr_base += 1;
        body.push(instr);
      }
    }
  }

  Ok(CalxFunc {
    name,
    params_types,
    ret_types,
    instrs: body,
  })
}

pub fn parse_instr(ptr_base: usize, node: &Cirru) -> Result<Vec<CalxInstr>, String> {
  match node {
    Cirru::Leaf(_) => Err(format!("expected expr of instruction, {}", node)),
    Cirru::List(xs) => {
      if xs.is_empty() {
        return Err(String::from("empty expr"));
      }
      let i0 = xs[0].to_owned();

      match i0 {
        Cirru::List(_) => Err(format!("expected instruction name in a string, got {}", i0)),
        Cirru::Leaf(name) => match name.as_str() {
          "local.get" => {
            if xs.len() != 2 {
              return Err(format!("local.get expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::LocalGet(idx)])
          }
          "local.set" => {
            if xs.len() != 2 {
              return Err(format!("local.set expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::LocalSet(idx)])
          }
          "local.tee" => {
            if xs.len() != 2 {
              return Err(format!("list.tee expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::LocalSet(idx)])
          }
          "global.get" => {
            if xs.len() != 2 {
              return Err(format!("global.get expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::GlobalGet(idx)])
          }
          "global.set" => {
            if xs.len() != 2 {
              return Err(format!("global.set expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::GlobalSet(idx)])
          }
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
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::BrIf(idx)])
          }
          "br" => {
            if xs.len() != 2 {
              return Err(format!("br expected a position, {:?}", xs));
            }
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::Br(idx)])
          }
          "block" => parse_block(ptr_base, xs, false),
          "loop" => parse_block(ptr_base, xs, true),
          "echo" => Ok(vec![CalxInstr::Echo]),
          "call" => {
            let name: String;
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
            }
            match &xs[1] {
              Cirru::Leaf(s) => name = s.to_owned(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            }

            Ok(vec![CalxInstr::Call(name)])
          }
          "call-import" => {
            let name: String;
            if xs.len() != 2 {
              return Err(format!("call expected function name, {:?}", xs));
            }
            match &xs[1] {
              Cirru::Leaf(s) => name = s.to_owned(),
              Cirru::List(_) => return Err(format!("expected a name, got {:?}", xs[1])),
            }

            Ok(vec![CalxInstr::CallImport(name)])
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
            let idx: usize;
            match &xs[1] {
              Cirru::Leaf(s) => {
                idx = parse_usize(s)?;
              }
              Cirru::List(_) => {
                return Err(format!("expected token, got {}", xs[1]));
              }
            }
            Ok(vec![CalxInstr::Quit(idx)])
          }
          "return" => Ok(vec![CalxInstr::Return]),
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

pub fn parse_block(ptr_base: usize, xs: &[Cirru], looped: bool) -> Result<Vec<CalxInstr>, String> {
  let mut p = ptr_base + 1;
  let mut chunk: Vec<CalxInstr> = vec![];
  let (params_types, ret_types) = parse_types(&xs[1])?;
  for (idx, line) in xs.iter().enumerate() {
    if idx > 1 {
      let instrs = parse_instr(p, line)?;
      for y in instrs {
        p += 1;
        chunk.push(y);
      }
    }
  }
  chunk.push(CalxInstr::BlockEnd);
  chunk.insert(
    0,
    CalxInstr::Block {
      looped,
      from: ptr_base + 1,
      to: p,
      params_types,
      ret_types,
    },
  );
  Ok(chunk)
}

pub fn parse_types(xs: &Cirru) -> Result<(Vec<CalxType>, Vec<CalxType>), String> {
  match xs {
    Cirru::Leaf(_) => Err(format!("expect expression for types, got {}", xs)),
    Cirru::List(ys) => {
      let mut params: Vec<CalxType> = vec![];
      let mut returns: Vec<CalxType> = vec![];
      let mut ret_mode = false;

      for x in ys {
        if let Cirru::Leaf(t) = x {
          match t.as_str() {
            "->" => {
              ret_mode = true;
            }
            "nil" => {
              if ret_mode {
                returns.push(CalxType::Nil);
              } else {
                params.push(CalxType::Nil);
              }
            }
            "bool" => {
              if ret_mode {
                returns.push(CalxType::Bool);
              } else {
                params.push(CalxType::Bool);
              }
            }
            "i64" => {
              if ret_mode {
                returns.push(CalxType::I64);
              } else {
                params.push(CalxType::I64);
              }
            }
            "f64" => {
              if ret_mode {
                returns.push(CalxType::F64);
              } else {
                params.push(CalxType::F64);
              }
            }
            "list" => {
              if ret_mode {
                returns.push(CalxType::List);
              } else {
                params.push(CalxType::List);
              }
            }
            "link" => {
              if ret_mode {
                returns.push(CalxType::Link);
              } else {
                params.push(CalxType::Link);
              }
            }
            a => return Err(format!("Unknown type: {}", a)),
          }
        }
      }

      Ok((params, returns))
    }
  }
}
