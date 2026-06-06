# Peri：修够了别人的 AI 工具，我们自己造了一个

我们团队拿到 [claude-code-best](https://github.com/claude-code-best/claude-code) 的代码就开始维护，算是从第一天就见识了 Vibe Slop Engineering——AI 生成的代码一层套一层，没人管架构，没人管性能，vibe 对了就行。跑着跑着内存就飙到好几个 G，到处是拼凑的胶水代码。

踩够了，于是我们决定自己造一个。Peri 是一个从零开始、用 Rust 写的 AI Agent 框架。用 Rust 的理由很简单——内存占用可控，启动快，不会再出现"工具本身就吃掉 2G 内存"的问题。27 万行代码，运行时只要 50MB。

值得一提的是，Peri 本身的代码，99% 是用国产模型生成的——主要用的是 DeepSeek-v4-pro 和 GLM-5.1。60 天完成，852 个 Commit。一个 AI 用另一个 AI 把自己写出来，这事本身就有意思。

项目地址：[GitHub](https://github.com/konghayao/peri)

## 快速上手

安装就一行命令。

macOS 和 Linux 用户直接跑这个，

```bash
curl -fsSL https://raw.githubusercontent.com/konghayao/peri/main/scripts/install.sh | bash
```

Windows 用户用 PowerShell，

```powershell
irm https://raw.githubusercontent.com/konghayao/peri/main/scripts/install.ps1 | iex
```

装完终端里输入 `peri` 就能跑起来。第一次打开会让你配模型和 API Key，在界面里填就行，不用去翻配置文件。我们就想让它尽可能简单，安装简单，配置简单，开箱即用。

## 我们对程序的追求

以下是 Peri 的 Agent 特性，每一个都是我们精心设计的，不是 AI Vibe 出来的 Slop

- 🧠 **内存可控** — 7 个 crate 严格分层，依赖方向单向无环，连续跑几个小时、几百轮对话，内存稳定在 200MB 左右，不会随会话长度增长

- ⚡ **Prompt Cache 98.5%** — 消息管线全链路受控，写入顺序、工具注册、动态占位符全部在会话开始时冻结，严格复用，大幅降低 token 费用和等待时间

- 🌐 **多模型支持** — 统一适配层封装 Anthropic 和 OpenAI 协议差异，DeepSeek、GLM、Qwen 等国产模型均可接入，运行时 `Ctrl+T` 一键切换

- 🛡️ **编译时安全网** — crate 边界是编译器强制的物理防火墙，AI 犯错被限制在单个 crate 内，类型错误和并发安全问题编译时拦截，不会跨模块传播

- 🔄 **Agent 自愈** — 类似 Rust 编译器的能力，不是简单报错，而是结构化地告诉 Agent 哪里错了、为什么错、该怎么改，引导它回到正确路径

- 🤖 **多 Agent 并发** — 支持同步、后台、fork 三种模式，多个子 Agent 可同时执行——并发搜索、正交验证、交叉开发，效率从串行等待变成并行推进

## 兼容 Claude Code 生态

这个大概是大家最关心的问题。切工具最怕什么？之前的配置全白费了，工作流得重新搭，团队成员还得重新适应。所以我们做的第一件事不是加新功能，而是确保 Peri 能直接接住 Claude Code 用户的所有积累。

`CLAUDE.md` 不用动，Skills 不用动，MCP 服务器不用动，插件也不用动。你在 Claude Code 里积累的项目配置、技能模板、第三方服务连接，Peri 全部原封不动地识别和使用。

插件系统也搬过来了。`/plugin` 命令照常用，在里面搜索、下载、安装插件，跟 Claude Code 的体验一模一样。之前装过的插件直接生效，不用重新装。

## 分层与扩展

Peri 的架构核心就一件事，把「Agent 能力」和「用户界面」彻底分开。

说真的，做这个分层一开始不是为了架构好看，是我们自己踩过坑。早期 Agent 和 UI 耦在一起，改 UI 不小心就把 Agent 逻辑改坏了，改 Agent 又把 UI 搞崩了。后来干脆一刀切开，各管各的，反而清爽了很多。

整个项目拆成了 7 个 crate，依赖方向严格单向、没有环。最底层是三个零依赖的独立库，`peri-lsp` 管 LSP 客户端，`peri-widgets` 管终端 UI 组件，`langfuse-client` 管遥测上报。往上走是 `peri-agent`，只管 ReAct 循环核心，不碰任何具体工具。再往上是 `peri-middlewares`，17 个中间件按固定顺序执行，文件系统、终端命令、MCP 服务连接、LSP 代码理解、HITL 权限审批、SubAgent 子任务派发，各自独立，每个只管一件具体的事。想加新能力就加一个中间件，想改某个行为就改对应的中间件，不会牵一发动全身。

再往上就是 `peri-acp`，ACP 服务层。ACP 对 AI Agent 的意义，跟 LSP 对编辑器的意义差不多——一个标准化的协议，让任何客户端都能接进来。终端 UI 只是一个客户端，Zed、VS Code 插件、Web 界面、自动化脚本，都是客户端，拿到的 Agent 能力完全一样。你想接什么前端都行，Agent 能力不会因为前端不同而打折扣。

这种设计不是一开始就规划好的，是 60 天开发过程中被现实逼出来的。模块边界越清晰，AI 生成代码时犯的错就越容易被限制在单个 crate 里，不会跨模块传播。

Peri 已经在我们团队的生产环境中使用了。如果你在用 Claude Code 或者对 AI Agent 框架感兴趣，欢迎试试看，遇到问题直接提 issue，我们会认真看每一条。

项目地址：[github.com/konghayao/peri](https://github.com/konghayao/peri)
