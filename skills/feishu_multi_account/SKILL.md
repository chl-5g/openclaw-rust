---
name: feishu_multi_account
description: 飞书多账号系统 - 支持为不同 Agent 绑定不同的飞书机器人账号
skill_type: account
version: 1.0.0
author: user
platform: feishu
---

# 飞书多账号系统 Skill

## 功能描述

这个 Skill 允许系统管理多个飞书机器人账号，并根据 Agent ID 自动路由消息到对应的账号。

## 目录结构

```
feishu_multi_account/
├── SKILL.md              # 本文件
├── references/           # 参考文档
│   └── api_docs.md       # 飞书 API 文档
├── scripts/              # 脚本工具
│   └── setup_bot.py      # 机器人初始化脚本
└── assets/              # 静态资源
    └── icon.png          # 机器人图标
```

## 使用方式

1. 在 `config/accounts.yaml` 中配置多个飞书账号
2. 每个账号绑定到特定的 agent_id
3. 系统会根据当前 Agent 自动选择对应的飞书账号发送消息

## 账号配置

参见 `config/accounts.yaml` 文件。

## 脚本使用

初始化机器人：
```bash
python scripts/setup_bot.py --app-id <APP_ID> --app-secret <APP_SECRET>
```

## 参考资料

- [飞书开放平台文档](references/api_docs.md)
