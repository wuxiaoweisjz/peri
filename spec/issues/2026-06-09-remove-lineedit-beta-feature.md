# 移除 LineEdit beta 特性

**状态**：Open
**优先级**：中
**创建日期**：2026-06-09

## 问题描述

LineEdit 作为 beta 功能上线后，经历了多轮迭代修复（V1 → V2 → V3 Diff-Apply），但持续面临本质性问题：bracket 验证误报、转义字符串匹配失败、连续编辑困惑、成功后高重读率等。tree-sitter 依赖已被移除。用户决定不再维护此功能，将其整体移除。

移除后，文件编辑回退到标准 `Edit` 工具（old_string 模型），betas 系统保留为空壳框架。

## 现状

LineEdit 通过 `settings.json` → `config.betas.lineEdit` 布尔值控制：

- `true`：`FilesystemMiddleware` 用 `LineEditTool` 替换 `EditFileTool`，工具列表中出现 "LineEdit" 而非 "Edit"
- `false`（默认）：使用标准 `Edit` 工具

涉及的核心模块（~2000+ 行代码）：

| 文件 | 行数（估） | 说明 |
|------|-----------|------|
| `peri-middlewares/src/tools/filesystem/line_edit.rs` | ~400 | V3 Diff-Apply 主入口，工具 trait 实现 |
| `peri-middlewares/src/tools/filesystem/line_edit_diff.rs` | ~250 | Unified diff 解析器 |
| `peri-middlewares/src/tools/filesystem/line_edit_match.rs` | ~600 | 5 级模糊匹配引擎 |
| `peri-middlewares/src/tools/filesystem/line_edit_verify.rs` | ~400 | 3 层验证（Diff Sanity + 括号平衡 + AST） |
| `peri-middlewares/src/tools/filesystem/line_edit_test.rs` | ~600 | 单元测试 |

相关已有 issue（均已完成或归档）：
- `spec/issues/2026-06-06-lineedit-bracket-false-positive.md`（Fixed）
- `spec/issues/2026-06-06-lineedit-consecutive-edits-confusion.md`（Open）
- `spec/issues/2026-06-06-lineedit-escape-and-context-matching-issues.md`（Fixed）
- `spec/issues/2026-06-07-lineedit-high-reread-rate-after-success.md`（Open）
- `spec/archive-issues/2026-06-06-lineedit-prompt-stress-testing.md`（已归档）

## 期望改进方向

1. 删除所有 LineEdit 工具源代码（5 个文件）
2. 移除 `BetasConfig.line_edit` 字段（`BetasConfig` 成为空结构体）
3. 移除 `FilesystemMiddleware` 的 `line_edit_mode` 开关，简化为始终使用 `EditFileTool`
4. 移除 agent builder 和 bg.rs 中对 `betas.line_edit` 的读取
5. 移除 TUI betas 面板中的 "lineEdit" 条目（`BETA_KEYS` 清空）
6. 移除 `core_tools.rs` 中的 `TOOL_LINE_EDIT` 常量
7. 更新 built-in agent `coder.md` 将 `LineEdit` 替换为 `Edit`
8. 删除 `prompts/lineedit_stress_test.txt`
9. 更新 `CLAUDE.md` 中 beta 功能表格

## 涉及文件

- `peri-middlewares/src/tools/filesystem/line_edit.rs` —— 删除
- `peri-middlewares/src/tools/filesystem/line_edit_diff.rs` —— 删除
- `peri-middlewares/src/tools/filesystem/line_edit_match.rs` —— 删除
- `peri-middlewares/src/tools/filesystem/line_edit_verify.rs` —— 删除
- `peri-middlewares/src/tools/filesystem/line_edit_test.rs` —— 删除
- `peri-middlewares/src/tools/filesystem/mod.rs` —— 移除 line_edit 模块声明和 `LineEditTool` 导出
- `peri-middlewares/src/middleware/filesystem.rs` —— 移除 `line_edit_mode` 字段、`with_line_edit_mode`、`tool_names_line_edit`、`LineEditTool` 导入
- `peri-middlewares/src/tool_search/core_tools.rs` —— 移除 `TOOL_LINE_EDIT` 常量
- `peri-acp/src/provider/config.rs` —— 移除 `BetasConfig.line_edit` 字段
- `peri-acp/src/agent/builder.rs` —— 移除对 `betas.line_edit` 的读取和 `with_line_edit_mode` 调用
- `peri-acp/src/session/command/bg.rs` —— 移除对 `betas.line_edit` 的读取
- `peri-tui/src/app/betas_panel.rs` —— 从 `BETA_KEYS` 移除 "lineEdit"，移除对应 match arm
- `peri-middlewares/src/subagent/built-in/coder.md` —— `LineEdit` → `Edit`
- `peri-middlewares/src/subagent/built_in_agents_test.rs` —— 更新断言
- `prompts/lineedit_stress_test.txt` —— 删除
- `CLAUDE.md` —— 更新 beta 功能表格

## 状态变更记录

| 日期 | 从 | 到 | 操作人 | 说明 |
|------|-----|-----|--------|------|
| 2026-06-09 | — | Open | agent | 创建 |

## 修复记录

（由 fix-issue 或 issue-verify skill 追加，创建时留空）
