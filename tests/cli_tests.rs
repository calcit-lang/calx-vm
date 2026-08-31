use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(path: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn calx(args: &[&str]) -> Output {
  Command::new(env!("CARGO_BIN_EXE_calx"))
    .args(args)
    .output()
    .expect("calx CLI must start")
}

fn output_text(bytes: &[u8]) -> String {
  String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn check_validates_without_executing_guest_code() {
  let source = fixture("tests/fixtures/check-no-run.cirru");
  let output = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  let stdout = output_text(&output.stdout);

  assert!(output.status.success(), "{}", output_text(&output.stderr));
  assert!(stdout.contains("[calx check] ok: 1 function(s), 2 syntax instruction(s)"));
  assert!(!stdout.lines().any(|line| line == "CHECK_MUST_NOT_EXECUTE"), "{stdout}");
}

#[test]
fn check_reports_validation_errors() {
  let source = fixture("tests/fixtures/invalid-type.cirru");
  let output = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  let stderr = output_text(&output.stderr);

  assert!(!output.status.success());
  assert!(stderr.contains("error[CALX_VALIDATION] validation"), "{stderr}");
  assert!(stderr.contains("invalid-type.cirru:4:3"), "{stderr}");
  assert!(stderr.contains("in function main at syntax[2]"), "{stderr}");
  assert!(stderr.contains("expected I64, found F64"), "{stderr}");
}

#[test]
fn check_reports_structured_parse_errors() {
  let source = fixture("tests/fixtures/invalid-parse.cirru");
  let output = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  let stderr = output_text(&output.stderr);

  assert!(!output.status.success());
  assert!(stderr.contains("error[CALX_PARSE_CIRRU] parse"), "{stderr}");
  assert!(stderr.contains("invalid-parse.cirru:2:"), "{stderr}");
  assert!(stderr.contains("Invalid indentation"), "{stderr}");
}

#[test]
fn run_reports_runtime_code_and_source_location() {
  let source = fixture("tests/fixtures/runtime-trap.cirru");
  let output = calx(&[source.to_str().expect("UTF-8 fixture path")]);
  let stderr = output_text(&output.stderr);

  assert!(!output.status.success());
  assert!(stderr.contains("error[CALX_RUNTIME_TRAP] runtime"), "{stderr}");
  assert!(stderr.contains("runtime-trap.cirru:4:3"), "{stderr}");
  assert!(stderr.contains("in function main at syntax[2]"), "{stderr}");
  assert!(stderr.contains("integer divide by zero"), "{stderr}");
}

#[test]
fn explain_filters_functions_and_shows_all_pipeline_layers() {
  let source = fixture("demos/if.cirru");
  let output = calx(&["explain", source.to_str().expect("UTF-8 fixture path"), "--function", "demo"]);
  let stdout = output_text(&output.stdout);

  assert!(output.status.success(), "{}", output_text(&output.stderr));
  assert!(stdout.contains("function demo ([I64] -> [])"), "{stdout}");
  assert!(!stdout.contains("function main ("), "{stdout}");
  assert!(stdout.contains("folded Cirru:"), "{stdout}");
  assert!(stdout.contains("syntax[001] If"), "{stdout}");
  assert!(stdout.contains("operand: [I64] -> []"), "{stdout}");
  assert!(stdout.contains("control:"), "{stdout}");
  assert!(stdout.contains("lowered: JmpIf(5)"), "{stdout}");
}

#[test]
fn check_and_explain_show_f64_comparison_types() {
  let source = fixture("tests/fixtures/f64-comparison.cirru");
  let checked = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  assert!(checked.status.success(), "{}", output_text(&checked.stderr));

  let explained = calx(&["explain", source.to_str().expect("UTF-8 fixture path")]);
  let stdout = output_text(&explained.stdout);
  assert!(explained.status.success(), "{}", output_text(&explained.stderr));
  assert!(stdout.contains("syntax[002] F64Lt"), "{stdout}");
  assert!(stdout.contains("operand: [F64, F64] -> [Bool]"), "{stdout}");
  assert!(stdout.contains("lowered: F64Lt"), "{stdout}");
}

#[test]
fn check_and_explain_show_strict_f64_buffer_pipeline() {
  let source = fixture("demos/f64-buffer.cirru");
  let checked = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  let check_stdout = output_text(&checked.stdout);
  assert!(checked.status.success(), "{}", output_text(&checked.stderr));
  assert!(
    check_stdout.contains("[calx check] ok: 1 function(s), 7 syntax instruction(s), strict typed"),
    "{check_stdout}"
  );

  let explained = calx(&["explain", source.to_str().expect("UTF-8 fixture path")]);
  let stdout = output_text(&explained.stdout);
  assert!(explained.status.success(), "{}", output_text(&explained.stderr));
  let golden = include_str!("fixtures/f64-buffer-explain.golden.txt");
  for expected in golden.lines().filter(|line| !line.is_empty()) {
    assert!(stdout.contains(expected), "missing `{expected}` in:\n{stdout}");
  }
}

#[test]
fn explain_rejects_an_unknown_function_filter() {
  let source = fixture("demos/if.cirru");
  let output = calx(&["explain", source.to_str().expect("UTF-8 fixture path"), "--function", "missing"]);
  let stderr = output_text(&output.stderr);

  assert!(!output.status.success());
  assert!(stderr.contains("unknown function `missing`"), "{stderr}");
}

#[test]
fn legacy_run_invocation_remains_supported() {
  let source = fixture("demos/hello.cirru");
  let output = calx(&[source.to_str().expect("UTF-8 fixture path")]);
  let stdout = output_text(&output.stdout);

  assert!(output.status.success(), "{}", output_text(&output.stderr));
  assert!(stdout.lines().any(|line| line == "hello world"), "{stdout}");
}

#[test]
fn typed_modules_use_the_strict_check_and_runtime_path() {
  let source = fixture("tests/fixtures/typed-module.cirru");
  let checked = calx(&["check", source.to_str().expect("UTF-8 fixture path")]);
  let check_stdout = output_text(&checked.stdout);
  assert!(checked.status.success(), "{}", output_text(&checked.stderr));
  assert!(check_stdout.contains("strict typed"), "{check_stdout}");

  let run = calx(&[source.to_str().expect("UTF-8 fixture path")]);
  let run_stdout = output_text(&run.stdout);
  assert!(run.status.success(), "{}", output_text(&run.stderr));
  assert!(run_stdout.contains("Value(I64(7))"), "{run_stdout}");
}
