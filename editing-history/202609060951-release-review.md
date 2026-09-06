# Clarify post-merge release review findings

## 中文

PR #64 合并后返回两条文档意见：编号实际指实现 PR，但未明确与 tracking Issue 区分，
现在分别链接 Issue #59 / PR #62 与 Issue #60 / PR #63；strict 拒绝所有 List，
不可暗示存在被接受的 typed List。中英文同步，不改变 VM 或版本。
独立临时 Calcit 0.13.77 副本通过候选 VM 的全部 18 项 Calx 回归；该 path override
仅是 unreleased 预检，不是正式版本采用，也未修改用户主工作区。

## English

Address two documentation findings returned after PR #64 merged. Distinguish
implementation PRs from tracking issues with explicit links (#59/#62 and #60/#63).
State that strict rejects every List value, with no implied typed-list exception.
No runtime/version change. An isolated Calcit 0.13.77 copy passed all 18 Calx tests
against the unreleased candidate via a temporary path override; this is not formal
version adoption and does not change the user's checkout.
