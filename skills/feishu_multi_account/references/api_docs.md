# 飞书开放平台 API 文档

## 基础 API

### 获取 tenant_access_token

```
POST https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal
```

请求体：
```json
{
  "app_id": "cli_xxxxx",
  "app_secret": "xxxxxxxx"
}
```

### 发送消息

```
POST https://open.feishu.cn/open-apis/im/v1/messages
```

Headers:
- Authorization: Bearer <tenant_access_token>

## 机器人配置

详见飞书开放平台官网：https://open.feishu.cn/
