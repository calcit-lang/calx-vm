//! Calx VM is a toy VM for learning WebAssembly.
//! It is a stack machine, and it is dynamically typed. Being an experiment, for Calcit project.

mod calx;
mod parser;
mod syntax;
mod util;
mod vm;

pub use calx::{Calx, CalxType};
pub use parser::{extract_nested, parse_function};
pub use syntax::CalxSyntax;
pub use util::log_calx_value;
pub use vm::{func::CalxFunc, instr::CalxInstr, instr::CALX_INSTR_EDITION, CalxImportsDict, CalxVM};
