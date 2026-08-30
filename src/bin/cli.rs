use std::env;
use std::fs;
use std::process;
use std::time::Instant;
use std::{collections::hash_map::HashMap, rc::Rc};

use argh::FromArgs;
use calx_vm::{
  log_calx_value, parse_program, trace_typed_validation, trace_validation, validate_program, Calx, CalxBoundaryType, CalxError,
  CalxHostBinding, CalxHostBindings, CalxImportDecl, CalxImportsDict, CalxProgram, CalxVM, ParsedProgram, ValidatedProgram,
  ValidationControlState, ValidationType,
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

fn main() {
  let result = match parse_args().command {
    Command::Run(args) => run(args),
    Command::Check(args) => check(args),
    Command::Explain(args) => explain(args),
  };
  if let Err(error) = result {
    eprintln!("{error}");
    process::exit(1);
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
  let program = load_program(&args.source)?;
  if uses_typed_module(&program) {
    return run_typed_module(args, program);
  }
  let mut vm = CalxVM::new(program.functions, vec![], standard_imports());
  let now = Instant::now();

  println!("[calx] start preprocessing");
  vm.preprocess(args.verbose)?;
  vm.setup_top_frame()?;

  if args.show_code {
    for func in vm.functions() {
      println!("loaded fn: {func}");
    }
  }

  println!("[calx] start running");
  match vm.run(vec![Calx::I64(1)]) {
    Ok(ret) => {
      println!("[calx] took {:.3?}: {ret:?}", now.elapsed());
      Ok(())
    }
    Err(error) => Err(error.to_string()),
  }
}

fn run_typed_module(args: RunArgs, program: ParsedProgram) -> Result<(), String> {
  let program = program.into_program().map_err(|error| error.to_string())?;
  let bindings = standard_typed_imports(program.imports())?;
  let mut vm = CalxVM::from_program(program, bindings).map_err(|error| error.to_string())?;
  let now = Instant::now();

  if args.show_code {
    for func in vm.functions() {
      println!("loaded fn: {func}");
    }
  }

  println!("[calx] start running typed program");
  let result = vm.run_typed(vec![]).map_err(|error| error.to_string())?;
  println!("[calx] took {:.3?}: {result:?}", now.elapsed());
  Ok(())
}

fn check(args: CheckArgs) -> Result<(), String> {
  let program = load_program(&args.source)?;
  if uses_typed_module(&program) {
    let instruction_count: usize = program.functions.iter().map(|func| func.syntax.len()).sum();
    let function_count = program.functions.len();
    let program = program.into_program().map_err(|error| error.to_string())?;
    ValidatedProgram::try_from_program(program).map_err(|error| error.to_string())?;
    println!("[calx check] ok: {function_count} function(s), {instruction_count} syntax instruction(s), strict typed");
    return Ok(());
  }
  let fns = program.functions;
  let instruction_count: usize = fns.iter().map(|func| func.syntax.len()).sum();
  validate_program(&fns, &[], &standard_imports()).map_err(|e| e.to_string())?;
  println!(
    "[calx check] ok: {} function(s), {instruction_count} syntax instruction(s)",
    fns.len()
  );
  Ok(())
}

fn explain(args: ExplainArgs) -> Result<(), String> {
  let program = load_program(&args.source)?;
  if uses_typed_module(&program) {
    return explain_typed(args, program);
  }
  let nodes = program.nodes;
  let fns = program.functions;
  let imports = standard_imports();
  let traces = trace_validation(&fns, &[], &imports).map_err(|e| e.to_string())?;
  let mut vm = CalxVM::new(fns, vec![], imports);
  vm.preprocess(false)?;

  let selected: Vec<usize> = match args.function {
    Some(name) => vec![vm
      .functions()
      .iter()
      .position(|func| func.name.as_ref() == name)
      .ok_or_else(|| format!("unknown function `{name}`"))?],
    None => (0..vm.functions().len()).collect(),
  };

  println!("[calx explain] {}", args.source);
  for index in selected {
    let func = &vm.functions()[index];
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
      if let Some(span) = &step.span {
        println!("    source: {span}");
      }
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

fn explain_typed(args: ExplainArgs, program: ParsedProgram) -> Result<(), String> {
  let nodes = program.nodes;
  let strict = CalxProgram::try_new(program.functions, program.globals, program.imports).map_err(|error| error.to_string())?;
  let traces = trace_typed_validation(&strict).map_err(|error| error.to_string())?;
  let validated = ValidatedProgram::try_from_program(strict).map_err(|error| error.to_string())?;
  let functions = validated.functions();
  let selected: Vec<usize> = match args.function {
    Some(name) => vec![functions
      .iter()
      .position(|function| function.name.as_ref() == name)
      .ok_or_else(|| format!("unknown function `{name}`"))?],
    None => (0..functions.len()).collect(),
  };

  println!("[calx explain] {}", args.source);
  for index in selected {
    print_function_explanation(&nodes[index], &functions[index], &traces[index])?;
  }
  Ok(())
}

fn load_program(source: &str) -> Result<ParsedProgram, String> {
  let contents = fs::read_to_string(source).map_err(|e| format!("failed to read `{source}`: {e}"))?;
  parse_program(source, &contents).map_err(|error| error.to_string())
}

fn uses_typed_module(program: &ParsedProgram) -> bool {
  let has_typed_locals = program
    .functions
    .iter()
    .flat_map(|function| function.locals.iter())
    .any(|local| matches!(local.value_type, CalxBoundaryType::Known(_)));
  has_typed_locals || !program.globals.is_empty() || !program.imports.is_empty()
}

fn standard_imports() -> CalxImportsDict {
  let mut imports: CalxImportsDict = HashMap::new();
  imports.insert(Rc::from("log"), (log_calx_value, 1));
  imports.insert(Rc::from("log2"), (log_calx_value, 2));
  imports.insert(Rc::from("log3"), (log_calx_value, 3));
  imports
}

fn standard_typed_imports(declarations: &[CalxImportDecl]) -> Result<CalxHostBindings, String> {
  let mut bindings = CalxHostBindings::new();
  for declaration in declarations {
    if !matches!(declaration.name.as_ref(), "log" | "log2" | "log3") {
      continue;
    }
    let params = declaration
      .params
      .iter()
      .map(|boundary| match boundary {
        CalxBoundaryType::Known(value_type) => Ok(*value_type),
        CalxBoundaryType::Dynamic => Err(format!("typed import `{}` cannot use Dynamic", declaration.name)),
      })
      .collect::<Result<Vec<_>, _>>()?;
    let binding = CalxHostBinding::void(params, log_calx_values_typed).map_err(|error| error.to_string())?;
    bindings.insert(declaration.name.clone(), binding);
  }
  Ok(bindings)
}

fn log_calx_values_typed(values: &[Calx]) -> Result<(), CalxError> {
  println!("log: {values:?}");
  Ok(())
}

fn print_function_explanation(
  node: &cirru_parser::Cirru,
  function: &calx_vm::CalxFunc,
  trace: &calx_vm::FunctionValidationTrace,
) -> Result<(), String> {
  println!(
    "\nfunction {} ({:?} -> {:?})",
    function.name, function.params_types, function.ret_types
  );
  println!("folded Cirru:\n{node}");
  println!("expanded validation and lowering:");
  for step in &trace.steps {
    let lowered = function
      .instrs
      .get(step.instruction_index)
      .ok_or_else(|| format!("missing lowered instruction at syntax[{}]", step.instruction_index))?;
    println!("  syntax[{:03}] {:?}", step.instruction_index, step.instruction);
    if let Some(span) = &step.span {
      println!("    source: {span}");
    }
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
  Ok(())
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
