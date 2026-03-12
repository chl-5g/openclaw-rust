---
name: openagentic_config
description: OpenAgentic 统一配置文件 - 定义 agents、channels、bindings、tools 等配置
skill_type: config
version: 1.0.0
author: user
---

# OpenAgentic 系统配置 Skill

## 功能描述

定义系统中所有的 Agent、通信通道、工具以及 Agent 之间的协作关系。

## 目录结构

```
openagentic_config/
├── SKILL.md              # 本文件
├── references/           # 参考文档
│   └── agent_roles.md    # Agent 角色说明
├── scripts/              # 配置生成脚本
│   └── validate.py       # 配置校验脚本
├── assets/               # 静态资源
│   └── architecture.png  # 系统架构图
└── config/
    └── agents.yaml       # Agent 配置（可选分离）
```

## 主要配置内容

### 1. Agents 配置
定义系统中所有的 Agent 及其角色、能力。

### 2. Channels 配置
定义系统支持的通信通道（飞书、Discord、Telegram 等）。

### 3. Tools 配置
定义系统可用的工具。

### 4. Bindings 配置
定义 Agent 与 Channel、Tool 的绑定关系。

### 5. AgentToAgent 配置
定义 Agent 之间的协作关系和任务派发规则。

## 详细配置

请参见 `config/agents.yaml`、`config/channels.yaml` 等文件。

## 脚本使用

校验配置：
```bash
python scripts/validate.py --config-dir ./config
```

## 参考资料

- [Agent 角色说明](references/agent_roles.md)
