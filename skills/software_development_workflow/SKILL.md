---
name: software_development_workflow
description: 软件开发工作流 - CEO → 产品负责人 → 工程师 → QA 端到端任务流转
skill_type: workflow
version: 1.0.0
author: user
---

# 软件开发工作流 Skill

## 功能描述

定义标准的软件开发流程，支持端到端的 Agent 任务流转。

## 目录结构

```
software_development_workflow/
├── SKILL.md              # 本文件
├── references/           # 参考文档
│   └── workflow_theory.md # 工作流理论
├── scripts/              # 工作流工具
│   └── init.py           # 初始化工作流
├── assets/               # 静态资源
│   └── flowchart.png    # 流程图
└── config/
    └── stages.yaml       # 阶段配置
```

## 工作流概述

```
CEO (规划)
  ↓
Product Manager (需求分析)
  ↓
Engineer (开发)
  ↓
QA (测试)
  ↓
CEO (审核) → 批准/拒绝
```

## Agent 定义

- **CEO**: 负责整体规划、任务分发、最终决策
- **Product Manager**: 负责需求分析、规格说明
- **Engineer**: 负责代码开发、单元测试
- **QA**: 负责测试验证、问题报告

## 阶段定义

详见 `config/stages.yaml`

## 脚本使用

初始化工作流：
```bash
python scripts/init.py --workflow software_dev
```

## 参考资料

- [工作流理论](references/workflow_theory.md)
