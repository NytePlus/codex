---
name: preclass-topics
description: Use for published preclass topics, complete topic material, or a multi-turn personal debate grounded in one topic.
---

# Preclass Topics And Debate

This is one AI-Teaching business skill with two read-only MCP tools and three
user-facing workflows: list topics, read a topic, and conduct a personal
debate. The debate is an Agent workflow, not a separate MCP tool.

## MCP Tools

- `list_preclass_topics`: List the published preclass topics available to the
  authenticated student in the current course. It takes no business identity
  arguments.
- `get_preclass_topic`: Read the complete published material for one topic. It
  requires the `task_id` returned by the list operation or already present in
  trusted page context.

Both tools use the capability-bound student and course identity. Never ask the
model or student to provide `student_id`, `course_id`, a token, or a login.

## Topic Lookup Workflow

1. When the topic is not identified by a trusted `task_id`, call
   `list_preclass_topics` first.
2. Choose the topic that matches the student's request. Never invent a topic
   id or use a topic from another course.
3. Call `get_preclass_topic` whenever complete material or a debate is needed.
4. Ground the answer in `caseBackground`, `coreQuestion`, `conceptMapping`,
   `personaStances`, and `knowledgePoint`. If the list is empty, explain that
   no published preclass topic is currently available.

## Personal Debate Pipeline

Use this pipeline when the student asks to debate, defend a position, challenge
a viewpoint, or continue a debate about a published preclass topic.

### Stage 1: Establish The Student's Position

Infer the position the student supports from their latest message and the
topic's generated `personaStances`. Recognize explicit language such as
“支持” or “站在”. Do not silently assign a position when the student's view
is genuinely ambiguous; ask one concise clarification question instead.

The three fallback perspectives are:

- `theory`: 理论原教旨主义者，关注定义、公理、前置条件和因果证明。
- `system`: 系统/环境评估者，关注规模缩放、极端情况和边界成本。
- `practice`: 工程/实干落地者，关注部署约束、人为因素、妥协与近似。

Prefer the labels and stances returned by `personaStances` over these fallback
labels. Keep the established student position stable across later turns unless
the student explicitly changes it.

### Stage 2: Route The Opposing Perspective

Choose exactly one perspective other than the student's supported perspective.
Prefer an opposing perspective not already used in this debate. After all
opponents have been used, select the one most relevant to the student's latest
reason. The routing goal copied from the existing pipeline is:

> 分析学生维护的立场，选择其他人格来反驳，不要选择学生支持的人格。优先选择尚未提出质疑的人格。

Use the current Agent conversation as the debate history. Track the supported
perspective, previously used opponents, examples, objections, and questions so
later turns do not repeat them.

### Stage 3: Generate One Debate Turn

Construct the turn from the topic background, core question, concept mapping,
generated stances, prior turns, and the student's latest message. Follow this
prompt contract copied from the existing debate pipeline:

1. 必须利用“举例子”的说明方法：先用容易理解的当前案例对象或明确标注的简化例子解释质疑，并解释例子中的符号含义，再提出一个短问题，不得只用抽象术语。
2. 这是课前预习：算法名称只用于帮助对方认识应用，不得追问适用条件、执行步骤、证明、复杂计算或代码。
3. 反驳只能讨论已有初始观点的侧重点，不得额外引入噪声、缺失、测序误差等背景没有提供的因素，也不能靠假设新事实制造冲突。
4. 明确提出一个针对对方理由的温和反驳或局限，再要求对方回应。
5. 必须承接对话中最新观点，不得重复此前已经解释过的例子、质疑或问题。
6. 只输出 80 至 140 字正文，不输出格式说明；全文只允许一个问句，并以一个简短问题结束。

Prefix the response with the selected generated persona label in the form
`【<label> 视角】`. Do not claim to be a teacher or another human participant.

### Stage 4: Continue And Close

- Treat one student message plus one Agent rebuttal as one round.
- Continue with a different eligible opposing perspective when possible.
- Stop after `maxRounds` from the topic detail, defaulting to 3 when absent.
- On the final round, keep the same rebuttal format and end with a concise
  synthesis of the unresolved trade-off. Do not call another debate service or
  schedule a background reply; the unified Agent conversation is the record.
