# Core teaching tutorials (#35)

- Add five short strict tutorials (integer arithmetic, conditions, loop/branch, calls/tail calls, traps) and one source-to-runtime walkthrough.
- Nine bounded source fixtures use typed locals to select the CLI's strict module path. They do not introduce VM syntax, values or public APIs.
- The tutorial integration test reads 17 command/expected-output pairs directly from Markdown; every tutorial has successful and failing commands. It executes the compiled CLI without invoking a shell and checks exit status plus key stdout/stderr excerpts, not unstable timing.
- Document folded Cirru expansion, I64/Bool contracts, branch labels, tail frame reuse, validation versus runtime failures and source locations. Link trace, diagnostics and Wasm mapping instead of duplicating their contracts.
- Keep Calcit Number/F64 and application eligibility distinct from VM I64 tutorials. This does not satisfy application adoption in Calcit #887 or prove a speedup.
- Repair outdated check/explain guidance that still called source spans future work and clarify legacy-only Dynamic behavior.
- Validation: cargo fmt --check; cargo test --offline; cargo clippy --offline --all-targets --all-features -- -D warnings; CARGO_NET_OFFLINE=true ./try.sh (all publish-workflow demos).
- Network access to the configured local proxy was unavailable on the first Cargo build; cached dependencies worked offline. No dependency or version changes were made.
