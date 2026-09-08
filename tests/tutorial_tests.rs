//! Execute the command/output pairs embedded in the core tutorials.
use std::{fs, path::Path, process::Command};

fn fenced<'a>(text: &'a str, language: &str) -> (&'a str, &'a str) {
  let opening = format!("```{language}\n");
  let (_, body) = text.split_once(&opening).expect("tutorial requires a fenced block");
  body.split_once("\n```").expect("tutorial fence must close")
}

#[test]
fn core_tutorial_commands_and_expected_output_stay_executable() {
  let root = Path::new(env!("CARGO_MANIFEST_DIR"));
  for name in ["integer-arithmetic", "conditionals", "loops", "functions", "traps", "end-to-end"] {
    let document = fs::read_to_string(root.join(format!("docs/tutorials/{name}.md"))).expect("tutorial exists");
    let mut successes = 0;
    let mut failures = 0;
    for case in document.split("<!-- cli-test: ").skip(1) {
      let (status, rest) = case.split_once(" -->").expect("test marker closes");
      let expect_success = match status {
        "success" => {
          successes += 1;
          true
        }
        "failure" => {
          failures += 1;
          false
        }
        _ => panic!("unknown tutorial test status: {status}"),
      };
      let (command, rest) = fenced(rest, "bash");
      let args = command
        .strip_prefix("cargo run --quiet -- ")
        .expect("documented command uses the local CLI");
      assert!(!args.contains('\n'), "one command per example");
      let (expected, _) = fenced(rest, "text");
      assert!(!expected.trim().is_empty(), "must assert meaningful output");
      let output = Command::new(env!("CARGO_BIN_EXE_calx"))
        .args(args.split_whitespace())
        .current_dir(root)
        .output()
        .expect("tutorial command starts");
      let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
      let stderr = String::from_utf8(output.stderr).expect("UTF-8 stderr");
      assert_eq!(output.status.success(), expect_success, "{name}: {command}\n{stdout}\n{stderr}");
      let actual = if expect_success { &stdout } else { &stderr };
      for line in expected.lines().filter(|line| !line.is_empty()) {
        assert!(actual.contains(line), "{name}: {command}\nmissing {line:?}\n{actual}");
      }
    }
    assert!(
      successes > 0 && failures > 0,
      "{name} needs both a working example and an error case"
    );
    assert_eq!(
      document.matches("```bash\n").count(),
      successes + failures,
      "{name}: every command must be tested"
    );
  }
}
