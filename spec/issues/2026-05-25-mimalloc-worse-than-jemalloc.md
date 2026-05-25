# mimalloc 替换后内存峰值反而高于 jemalloc，普通对话即达 100MB+

**状态**：Open
**优先级**：高
**类型**：性能
**创建日期**：2026-05-25

## 问题描述

按照 `2026-05-25-replace-jemalloc-with-mimalloc` 实施 mimalloc 替换 jemalloc 全局分配器后，内存峰值表现反而恶化。原 jemalloc 时期对话 RSS 尚在可控范围，切换 mimalloc 后普通对话场景（无需大量工具调用或长对话累积）即可冲到 100MB+，与替换预期的「更积极的内存归还策略缓解碎片化」目标相悖。

## 症状详情

| 维度 | 观察 |
|------|------|
| 峰值 RSS | 100MB+，比 jemalloc 同期明显更高 |
| 触发场景 | 普通对话即可触发，无需重度操作 |
| 与 jemalloc 对比 | jemalloc 时期同等场景 RSS 明显更低 |
| 对话规模 | 不需要多轮累积，几轮简单对话即上 100MB |
| 期望行为 | mimalloc 应该比 jemalloc 更积极归还内存，RSS 不应反超 |

## 复现条件

- **复现频率**：必现
- **触发步骤**：
  1. 启动 TUI（mimalloc 全局分配器）
  2. 发送普通对话消息（无需大量工具调用）
  3. 观察 RSS 快速达到 100MB+
- **环境**：macOS，Rust 2021

## 关联 Issue

- `spec/issues/2026-05-25-replace-jemalloc-with-mimalloc.md` —— 本次 mimalloc 替换的实施 issue（当前状态：Prepare）
- `spec/issues/2026-05-22-memory-linear-growth-no-compact.md` —— 原 jemalloc 内存线性增长问题，P3 第 13 项提及 mimalloc 作为备选方案

## 涉及文件

- `Cargo.toml` —— mimalloc workspace 依赖声明
- `peri-tui/Cargo.toml` —— mimalloc crate 依赖
- `peri-tui/src/main.rs` —— `#[global_allocator] static GLOBAL: mimalloc::MiMalloc`
- `peri-tui/src/app/thread_ops.rs` —— `alloc_collect()` 使用 `mi_collect(true)` 替代原 `jemalloc_decay()`
- `peri-tui/src/command/core/heapdump.rs` —— `/heapdump` 命令已迁移至 mimalloc 统计 API
