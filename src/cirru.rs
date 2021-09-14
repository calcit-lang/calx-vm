/*! conversion between Cirru and Clax
 *
 */

use lazy_static::lazy_static;
use regex::Regex;

use cirru_parser::Cirru;

use crate::primes::{Calx, CalxFunc, CalxInstr, CalxType};

pub fn parse_function(nodes: &Vec<Cirru>) -> Result<CalxFunc, String> {
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

  let mut params: Vec<CalxType> = vec![];
  if let Cirru::List(xs) = nodes[2].to_owned() {
    for x in xs {
      if let Cirru::Leaf(t) = x {
        match t.as_str() {
          "nil" => params.push(CalxType::Nil),
          "bool" => params.push(CalxType::Bool),
          "f64" => params.push(CalxType::F64),
          "list" => params.push(CalxType::List),
          "link" => params.push(CalxType::Link),
          a => return Err(format!("Unknown type: {}", a)),
        }
      }
    }
  } else {
    return Err(String::from("Expected params"));
  }

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
    params_type: params,
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
          "echo" => Ok(vec![CalxInstr::Echo]),
          "load" => {
            if xs.len() != 2 {
              return Err(format!("load takes exactly 1 argument, got {:?}", xs));
            }
            match &xs[1] {
              Cirru::Leaf(s) => {
                let p1 = parse_value(s)?;
                Ok(vec![CalxInstr::Load(p1)])
              }
              Cirru::List(a) => Err(format!("`load` not supporting list here: {:?}", a)),
            }
          }
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
