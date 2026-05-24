# 弹窗/面板输入框缺少统一的输入组件，能力参差不齐

**状态**：Fixed
**优先级**：中
**创建日期**：2026-05-24
**修复日期**：2026-05-24
**类型**：技术债

## 问题描述

项目中存在 5+ 种不同的输入框实现，能力严重不统一。主消息输入框（`TextArea`）支持完整的光标操作和跳词，但弹窗和面板中的输入框（AskUser 问答、Config 面板、Login 面板、Setup 向导、Plugin 搜索）功能残缺。用户在不同界面按下相同的快捷键，行为不一致甚至完全无响应。

## 症状详情

| 输入框位置 | 实现方式 | 缺失功能 |
|-----------|---------|---------|
| 主消息输入框 | `tui_textarea::TextArea`（`edit_utils.rs:177`） | 标准能力，全部支持 ✓ |
| AskUser 问答弹窗 | 裸 `String` + `usize` 光标 + `handle_edit_key()` | **Ctrl+Left/Right 跳词**、**Ctrl+W/Alt+Backspace 删词**、Alt+B/F 跳词、Ctrl+V 粘贴、鼠标点击定位 |
| Config 面板 | `handle_edit_key()` | 同 AskUser |
| Login 面板（Edit 模式） | `handle_edit_key()` + 手动 Ctrl+V | **Ctrl+Left/Right 跳词**、**Ctrl+W/Alt+Backspace 删词**、Alt+B/F 跳词、鼠标点击定位 |
| Setup 向导 | `handle_edit_key()` | 同 AskUser |
| Plugin 搜索框 | 裸 `insert(c)` + `backspace()`（`discover_search.rs`） | **所有光标操作均不支持**：无 Left/Right、无 Home/End、无 Delete、无跳词、无删词 |

**核心差距**：

1. **跳词能力**：主输入框支持 `Ctrl+Left/Right`（按词移动）和 `Alt+B/F`。所有弹窗输入框均无此能力。
2. **删词能力**：主输入框支持 `Ctrl+W`（删前一个词）和 `Alt+Backspace`。除 Plugin 搜索外，弹窗输入框只能逐字符删除。
3. **能力退化梯度**：主输入框 > Login/Config/Setup/AskUser（通过 `handle_edit_key`） > Plugin 搜索（最原始）

用户在各输入框间切换时期望一致的编辑体验，目前实际体验割裂严重。

## 期望改进方向

提供一个统一的单行输入组件（或增强现有 `handle_edit_key`），使所有弹窗/面板输入框获得与主消息输入框一致的基本编辑能力，至少包含：
- 跳词（Ctrl+Left/Right）
- 删词（Ctrl+W、Alt+Backspace）
- 复制粘贴（Ctrl+V）

## 涉及文件

- `peri-tui/src/app/edit_utils.rs`（158 行）—— 公共编辑辅助，`handle_edit_key` 缺少跳词/删词
- `peri-tui/src/app/ask_user_prompt.rs` —— 问答弹窗状态（裸 String + cursor）
- `peri-tui/src/app/ask_user_ops.rs` —— 问答弹窗操作
- `peri-tui/src/app/config_panel.rs` —— Config 面板
- `peri-tui/src/app/login_panel/component.rs` —— Login 面板
- `peri-tui/src/app/setup_wizard/ops.rs` —— Setup 向导
- `peri-tui/src/app/plugin_panel/handlers/plugin_handlers/discover_search.rs` —— Plugin 搜索（最原始）

## 修复方案

两条并行路径，各保持与光标模型一致的实现：

1. **`handle_edit_key()` 增强**：添加 `find_word_start`/`find_word_end` 辅助函数 + 4 个新 match arm（Ctrl+Left/Right、Ctrl+W、Alt+Backspace）。所有 `String` + `usize` 字符索引光标的弹窗（AskUser、Config、Login、Setup）自动受益。

2. **`InputState` 增强**（`peri-widgets`）：添加 `cursor_word_left`/`cursor_word_right`/`delete_word_backward` + 私有辅助 `byte_offset_at`。PluginPanel 的 discover_search 和 marketplace_add 处理器重写为使用这些方法，从裸 insert/backspace 升级为完整光标操作。

### 修复后能力对比

| 输入框位置 | 修��后支持 |
|-----------|-----------|
| 主消息输入框 | 全部 ✓（未改动） |
| AskUser / Config / Login / Setup | Char/Backspace/Delete/Left/Right/Home/End/Ctrl+A/E/K/U + **Ctrl+Left/Right 跳词** + **Ctrl+W/Alt+Backspace 删词** ✓ |
| Plugin 搜索框 / Marketplace 添加 | Char/Backspace/Delete/Left/Right/Home/End + **Ctrl+Left/Right 跳词** + **Ctrl+W/Alt+Backspace 删词** ✓ |

### 修改文件

- `peri-tui/src/app/edit_utils.rs` —— 辅助函数 + handle_edit_key 新增 4 个 arm
- `peri-tui/src/app/edit_utils_test.rs` —— **新建**，13 个测试
- `peri-widgets/src/input_field.rs` —— InputState 新增 4 个方法
- `peri-widgets/src/input_field_test.rs` —— 新增 5 个测试
- `peri-tui/src/app/plugin_panel/handlers/plugin_handlers/discover_search.rs` —— 重写为完整光标操作
- `peri-tui/src/app/plugin_panel/handlers/plugin_handlers/marketplace.rs` —— 重写 handle_marketplace_add
