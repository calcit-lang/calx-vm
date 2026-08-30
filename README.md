## Calx VM

> Calcit runner can be slow being a dynamic language. Calx VM trying to provide some helper tools for faster computation of very simple but repeated tasks. Ideally Calcit should use WASM for CPU heavy computations.

### Usages

Version 0.2.2 adds the semantic-safety baseline, typed validation,
`check`/`explain`, and lightweight error snapshots on top of the parser
compatibility and VM allocation/lookup improvements from 0.2.1. Native Calcit
bindings should use this stable crate version instead of a git hash.

0.2.2 在 0.2.1 的 parser compatibility 与 VM allocation/lookup 优化之上，
增加语义安全基线、类型验证、`check`/`explain` 和轻量错误快照。Calcit native
binding 应使用稳定 crate 版本，不要引用 git hash。

```bash
cargo install calx-vm
calx hello.cirru
calx run hello.cirru
calx check hello.cirru
calx explain hello.cirru
```

The first form remains a compatibility alias for `calx run`. `check` parses and
validates without lowering or executing guest code. `explain` shows folded
Cirru, expanded syntax, typed operand/control-stack transitions, and lowered
instructions; use `--function NAME` to focus on one function. 中文教程见
[`docs/tutorials/check-and-explain.md`](docs/tutorials/check-and-explain.md)。

it starts with a `main` function:

```cirru
fn main ()
  const 1
  call demo
  const 0
  call demo

fn demo (($a i64) ->)
  local.get $a

  if (->)
    do
      const 11
      echo
    do
      const 20
      echo
  const 3
  echo
```

`-s` to show instructions:

```bash
$ calx demos/if.cirru -s
[calx] start preprocessing
loaded fn: CalxFunc main (-> )
  00 Const(I64(1))
  01 Call(1)
  02 Const(I64(0))
  03 Call(1)

loaded fn: CalxFunc demo (I64 -> )
  local_names: 0_$a .
  00 LocalGet(0)
  01 JmpIf(5)
  02 Const(I64(20))
  03 Echo
  04 Jmp(8)
  05 Const(I64(11))
  06 Echo
  07 Jmp(8)
  08 Const(I64(3))
  09 Echo

[calx] start running
11
3
20
3
[calx] took 67.250µs: Nil
```

### Syntax Sugar

Code of:

```cirru
fn main ()
  i.add
    const 1
    i.mul
      const 2
      const 3

  echo
    dup

  assert "|expected 7"
    i.eq
      const 7
```

is desugared to:

```cirru
fn main ()
  const 2
  const 3
  i.mul
  const 1
  i.add

  dup
  echo

  const 7
  i.eq
  assert "|expected 7"
```

### Instructions

Find docs on https://docs.rs/calx_vm/ .

Highly inspired by:

- WASM https://github.com/WebAssembly/design/blob/main/Semantics.md
- Lox https://github.com/Darksecond/lox/blob/master/lox-vm/src/bettervm/vm.rs

### Preprocess

Before Calx runs instructions, it parses, validates, and lowers the program:

- folded Cirru is expanded into flat `CalxSyntax`;
- a typed operand/control-stack validator checks functions and structured control flow;
- `block`, `loop`, `if`, `br`, and `br-if` are lowered to executable internal instructions;
- branch and return instructions preserve declared results while discarding intermediate stack values.
- local variables are renamed to indexes

The codebase would be updated as I'm learning more about WASM.

### Development roadmap / 强化路线图

The teaching, validation, and Calcit compilation roadmap is maintained in
[`docs/roadmap.md`](docs/roadmap.md). 路线图正文以中文维护，issue 与 PR 使用中英双语。
The current experimental instruction contract is documented in
[`docs/instruction-set.md`](docs/instruction-set.md).
The per-opcode parser/validator/lowering/interpreter/test audit is maintained in
[`docs/instruction-matrix.md`](docs/instruction-matrix.md).
Typed validation and trap boundaries are specified in
[`RFCs/0001-validation-and-traps.md`](RFCs/0001-validation-and-traps.md).
Typed local, global, and host import module contracts are specified in
[`RFCs/0002-typed-boundaries.md`](RFCs/0002-typed-boundaries.md).
Source-aware parse, validation, runtime, and host diagnostics are documented in
[`docs/diagnostics.md`](docs/diagnostics.md).

### License

MIT
