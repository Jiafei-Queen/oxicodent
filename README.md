# Oxicodent — 增强型 Assistant 架构方案

[![许可证：MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)  [![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

> 本项目关注的是 **高效的人机协作** 与 **高质量的模型输出**，如果是想找能帮你自动写代码的 Agent，那可以划走了

Oxicodent 是一个 **增强型开发助手**，它通过 **注意力分离** 架构 
将架构思维与代码执行解耦。
- 它让开发者通过自然语言与 Reasoning 模块磨合设计
- 并由 Coder 模块在极净上下文中精准生成代码补丁

它不取代人的决策，而是通过 Markdown 工具链消除繁琐的搬砖工作。

**M.A.G.I. 三贤人设计**
- Multithreaded Async Granular Instruction
- - 多线程 异步 粒度化 指令
- Modular Attention Governance Interface
- - 模块化 注意力 治理 接口

## ⚠️ WARNING / 警告

```
本项目正在开发中，该应用现阶段 并不支持实质性的开发工作
```

## 开发进度
```
总体：
- 实现了基础的 API 配置文件读取并解析
- 实现了基础对话
- 支持开头对话提示词
- 支持 Markdown ```exec``` 命令执行
- 基本实现了 模型 连续工具调用支持
- 较为完善的 TUI 体验
- 实现了会话保持
- 实现了 read, Diff/Patch

架构：
- UI 主线程：负责 控制台I/O ，调用其他线程
- IO 线程：负责 网络I/O
- Worker 线程：负责命令执行

TODO:
- 实现三贤人架构
- 增加 Web Search
```

## 技术细节
### 1. 核心交互协议：隐式工具调用 (Implicit Tool Use)
放弃严苛的 JSON Schema，利用模型对 Markdown 代码块的天然生成能力。
- **```exec**: 拦截并执行 Shell 命令（ls, cargo build, npm test 等）。
- **```read**：专门用于读取文件，标有 **绝对行号**，提高 Diff 精准度
- **```diff:<filename>**: 应用补丁
- **/search:url（用户命令）**: 根据用户提供的网址，将 reqwest 扒下来的网页，使用 htmd 转化为 md 交给 Instruct 模块总结交给 Reasoning, 解决了传统 Web Search 命中率低的问题

### 2. 上下文管理策略 与 思考总结：三层过滤机制
- **项目总结（Project Summarise）**：在每次架构和设计、代码变动后更新 AI 的 `.oxicodent-summarise.md`
- **注意力分离（Attention Separation）**：在内部维护多份提示词和上下文，将 “工具调用”、“代码生成与应用”、“架构讨论” 等上下文解耦，保证模型在运行时的注意力完全集中于当前任务

---
### 架构优势
1. **模型兼容性**：转为小参数，量化模型设计。
2. **Token 效率**：通过主动压缩和按需调取，极大延长了有效上下文寿命。
3. **开发成本**：无需处理复杂的 Tool-calling API，只需字符串处理和进程控制。