# Oxicodent — 增强型 Assistant 架构方案

[![许可证：MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)  [![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

Oxicodent 是一个增强型开发助手。它通过‘注意力分离’架构，将架构思维与代码执行解耦。它让开发者通过自然语言与 Reasoning 模块磨合设计，并由 Coder 模块在极净上下文中精准生成代码补丁。它不取代人的决策，而是通过 Markdown 工具链消除繁琐的搬砖工作。

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
- 实现提示词与上下文解耦
- 实现 记忆索引
- 增加 Web Search
```

## 技术细节
### 1. 核心交互协议：隐式工具调用 (Implicit Tool Use)
放弃严苛的 JSON Schema，利用模型对 Markdown 代码块的天然生成能力。
- **```exec**: 拦截并执行 Shell 命令（ls, cargo build, npm test 等）。
- **```read**：专门用于读取文件，标有 **绝对行号**，提高 Diff 精准度
- **```diff:<filename>**: 应用补丁
- **```remem <ID>**: 调取历史存档中的详细执行结果或代码 Diff。
- **/search:url（用户命令）**: 根据用户提供的网址，将 reqwest 扒下来的网页，使用 htmd 转化为 md 放进 Memory Bank，解决了传统 Web Search 命中率低的问题

### 2. 上下文管理策略 与 思考总结：三层过滤机制
- **项目总结（Project Summarise）**：
- **注意力分离（Attention Separation）**：在内部维护多份提示词和上下文，将 “工具调用”、“代码生成与应用”、“架构讨论” 等上下文解耦，保证模型在运行时的注意力完全集中于当前任务

### 3. 人为干预
- **Session YAML**：将整个对话状态、记忆索引、项目目标持久化为可读的 `.assistant-history.yaml`。
- **断点续传**：重启程序后，模型通过读取 YAML 恢复“工程记忆”。
- **上帝视角 (Human-in-the-loop)**：用户可直接手动修改 YAML 文件中的“总结”或“目标”，纠正模型偏离的逻辑。
- **主动索取**：模型会在 用户需求模糊，项目意图不明确，API 接口不明确 等情况下，向用户索取信息，让 模型 拥有一定主导权

---
### 架构优势
1. **模型兼容性**：转为小参数，量化模型设计，使用 Reasoning 体验最佳，如：Qwen3-14B
2. **Token 效率**：通过主动压缩和按需调取，极大延长了有效上下文寿命。
3. **开发成本**：无需处理复杂的 Tool-calling API，只需字符串处理和进程控制。