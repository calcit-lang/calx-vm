## Calx VM

> Calcit runner can be slow being a dynamic language. Calx VM trying to provide some helper tools for faster computation of very simple but repeated tasks. Ideally Calcit should use WASM for CPU heavy computations.

### Usages

```bash
cargo install calx-vm
calx hello.cirru
```

it starts with a `main` function:

```cirru
fn main ()
  const "|hello world"
  echo
```

`-S` to show instructions:

```bash
calx -S hello.cirru
```

`-D` to disable preprocess:

```bash
calx -D hello.cirru
```

`--emit-binary filename` for encode functions into a binary file.

`--eval-binary` for reading a binary input file to run.

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

Highly inspired by:

- WASM https://github.com/WebAssembly/design/blob/main/Semantics.md
- Lox https://github.com/Darksecond/lox/blob/master/lox-vm/src/bettervm/vm.rs

For binary op, top value puts on right.

Calx Binary Edition `0.1`:

| Code                 | Usage                                                    | Note                                   |
| -------------------- | -------------------------------------------------------- | -------------------------------------- |
| `local.set $idx`     | pop from stack, set value at `$idx`                      |                                        |
| `local.tee $idx`     | set value at `$idx`, and also load it                    |                                        |
| `local.get $idx`     | get value at `$idx` load on stack                        |                                        |
| `local.new $name`    | increase size of array of locals                         | name is optional, defaults to `$<idx>` |
| `global.set $idx`    | set global value at `$idx`                               |                                        |
| `global.get $idx`    | get global value at `$idx`                               |                                        |
| `global.new`         | increase size of array of globals                        |                                        |
| `const $v`           | push value `$v` on stack                                 |                                        |
| `dup`                | duplicate top value on stack                             |                                        |
| `drop`               | drop top value from stack                                |                                        |
| `i.add`              | add two i64 numbers on stack into one                    |                                        |
| `i.mul`              | multiply two i64 numbers on stack into one               |                                        |
| `i.div`              | divide two i64 numbers on stack into one                 |                                        |
| `i.rem`              | calculate reminder two i64 numbers on stack into one     |                                        |
| `i.neg`              | negate i64 numbers on top of stack                       |                                        |
| `i.shr $bits`        | call SHR `$bits` bits on i64 numbers on top of stack     |                                        |
| `i.shl $bits`        | call SHL `$bits` bits on i64 numbers on top of stack     |                                        |
| `i.eq`               | detects if two i64 numbers on stack equal                |                                        |
| `i.ne`               | detects if two i64 numbers on stack not equal            |                                        |
| `i.lt`               | litter than, compares two i64 numbers on stack           |                                        |
| `i.gt`               | greater than, compares two i64 numbers on stack          |                                        |
| `i.le`               | litter/equal than, compares two i64 numbers on stack     |                                        |
| `i.ge`               | greater/equal than, compares two i64 numbers on stack    |                                        |
| `add`                | add two f64 numbers on stack into one                    |                                        |
| `mul`                | multiply two f64 numbers on stack into one               |                                        |
| `div`                | divide two f64 numbers on stack into one                 |                                        |
| `neg`                | negate f64 numbers on top of stack                       |                                        |
| `list.new`           |                                                          | TODO                                   |
| `list.get`           |                                                          | TODO                                   |
| `list.set`           |                                                          | TODO                                   |
| `link.new`           |                                                          | TODO                                   |
| `and`                |                                                          | TODO                                   |
| `or`                 |                                                          | TODO                                   |
| `not`                |                                                          | TODO                                   |
| `br $n`              | branch `$n` level of block, 0 means end of current block |                                        |
| `br-if $n`           | like `br $n` but detects top value on stack first        | Internal                               |
| (JMP `$l`)           | jump to line `$l`                                        | Internal                               |
| (JMP_IF `$l`)        | conditional jump to `$l`                                 |
| `block $types $body` | declare a block                                          |                                        |
| `loop $types $body`  | declare a loop block                                     |                                        |
| (BlockEnd)           | internal mark for ending a block                         | Internal                               |
| `echo`               | pop value from stack and print                           |                                        |
| `call $f`            | call function `$f`                                       |                                        |
| `return-call $f`     | tail call function `$f`                                  |                                        |
| `call-import $f`     | call imported function `$f`                              |                                        |
| `unreachable`        | throw unreachable panic                                  |                                        |
| `nop`                | No op                                                    |                                        |
| `quit $code`         | quit program and return exit code `$code`                |                                        |
| `return`             |                                                          | TODO                                   |
| `fn $types $body`    |                                                          | Global definition                      |
| `assert`             | `quit(1)` if not `true`                                  | for testing                            |

For `$types`, it can be `($t1 $t2 -> $t3 $t4)`, where supported types are:

- nil
- i64
- f64
- bool _TODO_
- str _TODO_
- list _TODO_
- link _TODO_

### License

MIT
