#!/usr/bin/env bash
set -euo pipefail

run_demo() {
  local source="$1"
  local expected="$2"
  local output

  output="$(cargo run --quiet -- -s "$source")"
  printf '%s\n' "$output"

  if [[ "$output" != *"$expected"* ]]; then
    printf 'demo %s did not contain expected output %q\n' "$source" "$expected" >&2
    return 1
  fi
}

check_demo() {
  local source="$1"
  local expected="$2"
  local output

  output="$(cargo run --quiet -- check "$source")"
  printf '%s\n' "$output"

  if [[ "$output" != *"$expected"* ]]; then
    printf 'check %s did not contain expected output %q\n' "$source" "$expected" >&2
    return 1
  fi
}

run_demo demos/hello.cirru "hello world"
run_demo demos/sum.cirru $'\n500000500000\n'
run_demo demos/assert.cirru ": Nil"
run_demo demos/nested.cirru $'\n7\n'
run_demo demos/named.cirru $'\n103\n'
run_demo demos/recur.cirru $'\n200000010000000\n'
run_demo demos/fibonacci.cirru $'\n5702887\n'
run_demo demos/if.cirru $'\n11\n3\n20\n3\n'
run_demo demos/fibo-if.cirru $'\n5702887\n'
check_demo demos/f64-buffer.cirru "[calx check] ok: 1 function(s), 7 syntax instruction(s), strict typed"
