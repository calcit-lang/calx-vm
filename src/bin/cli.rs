use std::env;
use std::fs;
use std::process;
use std::time::Instant;
use std::{collections::hash_map::HashMap, rc::Rc};

use argh::FromArgs;
use cirru_parser::{parse, Cirru};

use calx_vm::{
  log_calx_value, parse_function, trace_validation, validate_program, Calx, CalxFunc, CalxImportsDict, CalxVM, ValidationControlState,
  ValidationType,
};

#[derive(FromArgs)]
/// run and inspect Calx programs
struct TopLevel {
  #[argh(subcommand)]
  command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
  Run(RunArgs),
  Check(CheckArgs),
  Explain(ExplainArgs),
}

#[derive(FromArgs)]
/// run a Calx program
#[argh(subcommand, name = "run")]
struct RunArgs {
  /// show lowered code
  #[argh(switch, short = 's')]
  show_code: bool,
  /// show verbose preprocessing output
  #[argh(switch, short = 'v')]
  verbose: bool,
  /// cirru source file
  #[argh(positional)]
  source: String,
}

#[derive(FromArgs)]
/// parse and validate a Calx program without executing it
#[argh(subcommand, name = "check")]
struct CheckArgs {
  /// cirru source file
  #[argh(positional)]
  source: String,
}

#[derive(FromArgs)]
/// explain validation and lowering for a Calx program
#[argh(subcommand, name = "explain")]
struct ExplainArgs {
  /// only explain the named function
  #[argh(option)]
  function: Option<String>,
  /// cirru source file
  #[argh(positional)]
  source: String,
}

fn main() -> Result<(), String> {
  match parse_args().command {
    Command::Run(args) => run(args),
    Command::Check(args) => check(args),
    Command::Explain(args) => explain(args),
  }
}

fn parse_args() -> TopLevel {
  let mut args: Vec<String> = env::args().collect();
  if args.len() > 1 && !matches!(args[1].as_str(), "run" | "check" | "explain" | "--help" | "help") {
    args.insert(1, "run".to_string());
  }

  let command_name = args.first().map(String::as_str).unwrap_or("calx");
  let values: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
  TopLevel::from_args(&[command_name], &values).unwrap_or_else(|early_exit| {
    match early_exit.status {
      Ok(()) => println!("{}", early_exit.output),
      Err(()) => eprintln!("{}\nRun {command_name} --help for more information.", early_exit.output),
    }
    process::exit(if early_exit.status.is_ok() { 0 } else { 1 });
  })
}

fn run(args: RunArgs) -> Result<(), String> {
  let (_, fns) = load_program(&args.source)?;
  let mut vm = CalxVM::new(fns, vec![], standard_imports());
  let now = Instant::now();

  println!("[calx] start preprocessing");
  vm.preprocess(args.verbose)?;
  vm.setup_top_frame()?;

  if args.show_code {
    for func in &vm.funcs {
      println!("loaded fn: {func}");
    }
  }

  println!("[calx] start running");
  match vm.run(vec![Calx::I64(1)]) {
    Ok(ret) => {
      println!("[calx] took {:.3?}: {ret:?}", now.elapsed());
      Ok(())
    }
    Err(e) => {
      println!("VM state: {:?}", vm.stack);
      println!("{e}");
      Err(String::from("Failed to run."))
    }
  }
}

fn check(args: CheckArgs) -> Result<(), String> {
  let (_, fns) = load_program(&args.source)?;
  let instruction_count: usize = fns.iter().map(|func| func.syntax.len()).sum();
  validate_program(&fns, &[], &standard_imports()).map_err(|e| e.to_string())?;
  println!(
    "[calx check] ok: {} function(s), {instruction_count} syntax instruction(s)",
    fns.len()
  );
  Ok(())
}

fn explain(args: ExplainArgs) -> Result<(), String> {
  let (nodes, fns) = load_program(&args.source)?;
  let imports = standard_imports();
  let traces = trace_validation(&fns, &[], &imports).map_err(|e| e.to_string())?;
  let mut vm = CalxVM::new(fns, vec![], imports);
  vm.preprocess(false)?;

  let selected: Vec<usize> = match args.function {
    Some(name) => vec![vm
      .funcs
      .iter()
      .position(|func| func.name.as_ref() == name)
      .ok_or_else(|| format!("unknown function `{name}`"))?],
    None => (0..vm.funcs.len()).collect(),
  };

  println!("[calx explain] {}", args.source);
  for index in selected {
    let func = &vm.funcs[index];
    let trace = &traces[index];
    println!("\nfunction {} ({:?} -> {:?})", func.name, func.params_types, func.ret_types);
    println!("folded Cirru:\n{}", nodes[index]);
    println!("expanded validation and lowering:");

    for step in &trace.steps {
      let lowered = func
        .instrs
        .get(step.instruction_index)
        .ok_or_else(|| format!("missing lowered instruction at syntax[{}]", step.instruction_index))?;
      println!("  syntax[{:03}] {:?}", step.instruction_index, step.instruction);
      println!(
        "    operand: {} -> {}",
        format_types(&step.operand_stack_before),
        format_types(&step.operand_stack_after)
      );
      println!(
        "    control: {} -> {}",
        format_controls(&step.control_stack_before),
        format_controls(&step.control_stack_after)
      );
      println!("    lowered: {lowered:?}");
    }
  }
  Ok(())
}

fn load_program(source: &str) -> Result<(Vec<Cirru>, Vec<CalxFunc>), String> {
  let contents = fs::read_to_string(source).map_err(|e| format!("failed to read `{source}`: {e}"))?;
  let nodes = parse(&contents).map_err(|e| format!("failed to parse `{source}`: {e}"))?;
  let fns = nodes
    .iter()
    .map(|node| match node {
      Cirru::List(items) => parse_function(items),
      Cirru::Leaf(_) => Err("expected top-level function expressions".to_string()),
    })
    .collect::<Result<Vec<_>, _>>()?;
  Ok((nodes, fns))
}

fn standard_imports() -> CalxImportsDict {
  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(Rc::from("log"), (log_calx_value, 1));
  imports.insert(Rc::from("log2"), (log_calx_value, 2));
  imports.insert(Rc::from("log3"), (log_calx_value, 3));
  imports
}

fn format_types(types: &[ValidationType]) -> String {
  format!("[{}]", types.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))
}

fn format_controls(controls: &[ValidationControlState]) -> String {
  format!(
    "[{}]",
    controls
      .iter()
      .map(|frame| {
        format!(
          "{:?}(height={}, label={}, {})",
          frame.kind,
          frame.height,
          format_types(&frame.label_types),
          if frame.unreachable { "unreachable" } else { "reachable" }
        )
      })
      .collect::<Vec<_>>()
      .join(" > ")
  )
}
