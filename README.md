## Calx VM

> Calcit runner can be slow being a dynamic language. Calx VM trying to provide some helper tools for faster computation of very simple but repeated tasks. Ideally Calcit should use WASM for CPU heavy computations.

### Usages

Version 0.2.1 includes the parser ownership compatibility fixes and VM
allocation/lookup improvements accumulated on `main` after 0.2.0. Native
Calcit bindings should use this stable crate version instead of a git hash.

0.2.1 包含 0.2.0 之后在 `main` 积累的 parser ownership 兼容修复和 VM
allocation/lookup 优化。Calcit native binding 应使用稳定 crate 版本，不要引用
git hash。

```bash
cargo install calx-vm
calx hello.cirru
```

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
  01 Call("demo")
  02 Const(I64(0))
  03 Call("demo")

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

Before Calx running the instructions, Calx performs preprocessing to them. There are several tasks:

- `block` and `loop` are expanded since there are `block-end` instructions
- `br` and `br-if` also expanded to `jmp` and `jmp-if` instructions, internally
- stack size is checked to ensure it's consistent among branches, and tidied up at function end
- local variables are renamed to indexes

The codebase would be updated as I'm learning more about WASM.

### Development roadmap / 强化路线图

The teaching, validation, and Calcit compilation roadmap is maintained in
[`docs/roadmap.md`](docs/roadmap.md). 路线图正文以中文维护，issue 与 PR 使用中英双语。
The current experimental instruction contract is documented in
[`docs/instruction-set.md`](docs/instruction-set.md).

### License

MIT
