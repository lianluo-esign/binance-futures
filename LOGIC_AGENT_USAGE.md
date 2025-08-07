# 如何在 Claude Code 中使用 Logic Light Fire 模板

## 方法 1：使用 general-purpose agent 并提供详细指令

当需要逻辑分析和方案设计时，可以这样调用：

```
使用 Task 工具，选择 general-purpose agent，
在 prompt 中明确要求：
"按照 logic-light-fire.md 模板的分析框架，
分析这个问题并提供解决方案设计，
不要编写代码，只提供架构和设计思路"
```

## 方法 2：在 CLAUDE.md 中集成

可以将 logic-light-fire.md 的核心内容添加到 CLAUDE.md 文件中，
这样所有的交互都会考虑这些设计原则。

## 方法 3：作为参考文档

在对话中明确引用：
"请参考 logic-light-fire.md 的分析框架来处理这个问题"

## 示例用法

### 架构设计请求
"我需要设计一个高性能的实时数据处理系统，
请使用 logic-light-fire 的分析框架，
提供架构设计方案（不需要代码实现）"

### 创造性解决方案
"创造力 - 设计一个新颖的用户交互方式，
让交易数据可视化更直观"

### Bug 分析
"系统在高并发时出现数据不一致，
请分析可能的原因并提供修复策略"

## 注意事项

1. Claude Code 的 sub-agent 是预定义的，不能直接加载自定义模板
2. 但可以通过详细的 prompt 指令让 agent 遵循你的模板逻辑
3. 将模板内容整合到 CLAUDE.md 可以让其成为项目规范的一部分