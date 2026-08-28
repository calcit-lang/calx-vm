# 维护指南 / Maintainer guide

## 中文

- 提交前运行 `cargo fmt --check`、`cargo test` 和仓库 publish workflow 中列出的全部 demos。
- parser ownership 类型变化必须同时验证 crates.io 无 lockfile 构建，避免宽松 semver 范围吸收不兼容版本后使已发布 crate 无法编译。
- 公开 VM API 的 `CalxError` 目前较大；严格 Clippy 的 `result_large_err` 是已知 API 设计债务，后续应单独评估 boxed error 的兼容成本。

## English

- Before committing, run `cargo fmt --check`, `cargo test`, and every demo listed by the publish workflow.
- Parser ownership changes must also be checked in a crates.io-style build without relying on a stale lockfile, so a broad semver range cannot break a published crate.
- The public VM API currently has a large `CalxError`; strict Clippy's `result_large_err` is known API-design debt and boxed-error compatibility should be evaluated separately.
