# 测试分离规范债务：89.8% 源文件仍内联测试

**状态**：Open
**优先级**：中
**创建日期**：2026-05-14

## 问题描述

项目规范要求测试与源码分离为同目录 `_test.rs` 文件（≥30 行必须分离），但实际执行率仅 10.2%。137 个含测试的源文件中，113 个仍使用 `#[cfg(test)]` 内联测试，其中 73 个内联测试超过 100 行。

## 现状数据

### 按严重程度分布

| 严重级别 | 阈值 | 文件数 |
|---------|------|--------|
| CRITICAL | >100 行，必须分离 | 73 |
| MAJOR | 30-100 行，应分离 | 40 |
| INFO | <30 行，可保持 | 6 |
| 已分离 | `*_test.rs` 文件 | 24 |

### 按 Crate 分布

| Crate | CRITICAL | MAJOR | 已分离 | 违规率 |
|-------|----------|-------|--------|--------|
| rust-create-agent | 14 | 7 | 7 | 75% |
| rust-agent-middlewares | 34 | 15 | 8 | 86% |
| rust-agent-tui | 19 | 16 | 7 | 83% |
| perihelion-widgets | 7 | 12 | 0 | 100% |
| langfuse-client | 2 | 2 | 1 | 80% |
| perihelion-lsp | 3 | 3 | 0 | 100% |

### Top 10 内联测试最严重文件

| 文件 | 内联测试行数 |
|------|-------------|
| ~~`rust-create-agent/src/llm/anthropic.rs`~~ | ~~729~~ **已分离** |
| ~~`rust-agent-middlewares/src/tools/filesystem/grep.rs`~~ | ~~489~~ **已分离** |
| ~~`rust-create-agent/src/agent/compact/micro.rs`~~ | ~~457~~ **已分离** |
| ~~`rust-create-agent/src/agent/token.rs`~~ | ~~452~~ **已分离** |
| ~~`rust-create-agent/src/agent/compact/re_inject.rs`~~ | ~~413~~ **已分离** |
| ~~`rust-agent-tui/src/config/types.rs`~~ | ~~401~~ **已分离** |
| ~~`rust-create-agent/src/agent/compact/full.rs`~~ | ~~400~~ **已分离** |
| ~~`rust-agent-tui/src/prompt.rs`~~ | ~~383~~ **已分离** |
| ~~`rust-agent-middlewares/src/hitl/mod.rs`~~ | ~~376~~ **已分离** |
| ~~`rust-agent-tui/src/ui/message_view.rs`~~ | ~~359~~ **已分离** |

Top 10 全部完成分离（合计 3891 行 → 10 个 `*_test.rs` 文件）。

### 已正确分离的文件（24 个）

已有 24 个文件使用 `#[path = "..._test.rs"] mod tests;` 模式正确分离，可作为参考模板。无重复测试（已分离的文件不再含内联测试）。

## 期望改进方向

分批将内联测试迁移到 `*_test.rs` 文件，使用 `#[path = "..._test.rs"] mod tests;` 引用模式（与已有 24 个文件一致）。

Top 10 批次已完成。后续按 crate 逐个清零：

- rust-create-agent（14 CRITICAL + 7 MAJOR 待处理）
- rust-agent-middlewares（34 CRITICAL + 15 MAJOR 待处理）
- rust-agent-tui（19 CRITICAL + 16 MAJOR 待处理）
- perihelion-widgets（7 CRITICAL + 12 MAJOR 待处理）
- langfuse-client（2 CRITICAL + 2 MAJOR 待处理）
- perihelion-lsp（3 CRITICAL + 3 MAJOR 待处理）

## 涉及文件

- 73 个 CRITICAL 文件（完整列表见 `spec/reviews/2026-05-14.md`）
- 40 个 MAJOR 文件（完整列表见 `spec/reviews/2026-05-14.md`）
