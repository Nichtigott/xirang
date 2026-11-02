# XiRang 与 CodeGraph / XiRang and CodeGraph

[中文首页](../README.zh-CN.md) · [English](../README.md) · [评测工具 / Benchmark tooling](../bench/README.md)

## 比较什么 / What to compare

CodeGraph 的语法树、符号解析和调用关系分析已经能帮助理解源码。息壤也需要语义分析；注册不能取代类型推导，不能让动态调用天然变成确定调用。

CodeGraph uses syntax trees, symbol resolution and call relationships to help understand code. XiRang also needs semantic analysis. Registration does not replace type inference or make dynamic targets inherently certain.

息壤增加的是显式工程合同及其消费者：对象注册、父级规则、graft、分区、结构迁移、候选范围、验证计划、审查包和项目知识。优势主张是这些机制的**整合**，不是“所有查询更准”“所有模型更省”，也不是其他工具原则上无法实现类似工作流。

XiRang adds declared engineering contracts and operations that consume them: registration, parent rules, grafting, partitioning, migration, scoped candidates, verification, review and project knowledge. The differentiator is their integration, not a universal accuracy or cost claim or an assertion that other tools cannot build equivalent workflows.

## 已记录的实验 / Recorded experiments

以下摘录自 2026-11-01 的本地双臂报告，**本次 README 更新未重新运行这些比较**。原始会话与查询 wire 日志属于本地评测产物，没有随本文发布；这些数字是有明确范围的历史记录，不是当前检出的独立可复验基准认证。

The following summarizes a local report dated 2026-11-01. **These comparisons were not rerun for this README update.** Raw sessions and query-wire logs are local artifacts and are not distributed with this page. These are scoped historical measurements, not independently reproducible certification of the current checkout.

### 查询覆盖 / Query evidence coverage

| 项 / Item | 记录 / Recorded result |
| --- | --- |
| 装置 / Apparatus | 15 questions: 8 synthetic + 7 ledger; XiRang p61, CodeGraph 1.6.1 |
| 判据 / Scoring | 指定证据文本是否出现在回答 / Required evidence text appears in the response |
| XiRang | 43 / 43 evidence items |
| CodeGraph | 37 / 43 evidence items |
| 回复大小 / Response bytes | Synthetic: 19.1 KB vs 7.6 KB; ledger: 47.5 KB vs 35.7 KB |

这说明指定题目里息壤带回更多要求项，也付出了更长回复。它没有测量完整调用图的准确率/召回率，没有证明模型会正确使用这些证据，也没有证明 token 花费更低。

XiRang returned more requested evidence on these questions, in larger replies. This does not measure full-callgraph precision/recall, establish correct model use of the evidence, or demonstrate lower token cost.

### 真实 Agent pilot / Agent pilot

| 项 / Item | 记录 / Recorded result |
| --- | --- |
| 装置 / Apparatus | 3 cases × 2 arms × 1 repeat = 6 sessions; deepseek-v4.1-flash |
| 入口 / Interface | CLI wrappers, not the formal MCP attachment protocol |
| 机械成功 / Mechanical success | XiRang 3/3; CodeGraph 3/3 |
| 验收 / Checks | Normal and replacement behavior; protected files unchanged |
| 输入成本当量 / Input cost-equivalent | XiRang 4,919,792; CodeGraph 4,122,470; ratio 1.19× |
| 公式 / Formula | Uncached input × 10 + cached reads; output counted separately |
| 模型步骤 / Model steps | 68 vs 65 |
| 工具调用 / Tool calls | 90 vs 89 |

该 pilot 的 XiRang 二进制早于查询表中的 p61；有 harness 提示与记忆注入，只有一次重复，无方差估计。不能把两套装置拼成“质量提高且成本下降”的结论。CodeGraph 对照版本是 1.6.1，不代表后续版本。

The pilot used an earlier XiRang binary than the p61 query experiment, injected harness context and memory, and had one repeat with no variance estimate. Do not combine these experiments into a claim of better quality and lower cost. CodeGraph 1.6.1 is not a claim about subsequent versions.

### 正式新版实验 / New formal run

登记版设计为 18 会话。准备与反面控制已运行，但模型阶段因环境/凭据条件未成功启动，记录为 **NOT_RUN**，不能计入模型效果或成本。

The registered design has 18 sessions. Preparation and controls ran, but environment/credential failures prevented model execution. Its status is **NOT_RUN**, not an effectiveness result.

## 用户应如何判断 / How to choose

- 只想理解现有仓库：比较符号查找、调用关系、跨文件追踪、新鲜度和工具响应。Source exploration should be judged on symbol/call resolution, cross-file context and freshness.
- 想限制 Agent 改动：检查候选范围、保护内容、计划失效、审查与落地授权是否真的参与写入。For bounded changes, test actual scope, protection, freshness, review and landing enforcement.
- 想维护长期知识：测试换会话、搬对象、修订规范后，旧依据能否被正确续接或失效。For durable knowledge, test session changes, migrations and revised policies.
- 想替换或分区混合语言组件：验证真实构建、运行树、消费者行为和反面用例，不只比查询文本。For mixed components, verify builds, runtime trees, consumers and negative controls.

[六类问题的实现与边界](ai-engineering-workflow.md)给出源码入口。公开评测工具位于 [bench](../bench)，但工具存在不等于上述原始运行日志已经随仓库提供；新的性能或效果承诺需要重新固定版本、任务、判据与原始证据。

See [the workflow guide](ai-engineering-workflow.md) for implementation references. Public tooling is in [bench](../bench); its presence does not imply that the raw runs above are distributed. New claims require pinned versions, tasks, criteria and raw evidence.
