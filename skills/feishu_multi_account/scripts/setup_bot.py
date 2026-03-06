#!/usr/bin/env python3
"""
飞书机器人初始化脚本
"""

import argparse
import json
import requests

def setup_bot(app_id: str, app_secret: str):
    """初始化飞书机器人"""
    
    # 获取 tenant_access_token
    url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal"
    data = {
        "app_id": app_id,
        "app_secret": app_secret
    }
    
    response = requests.post(url, json=data)
    result = response.json()
    
    if result.get("code") == 0:
        token = result.get("tenant_access_token")
        print(f"获取 token 成功: {token[:20]}...")
        return token
    else:
        print(f"获取 token 失败: {result.get('msg')}")
        return None

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="飞书机器人初始化")
    parser.add_argument("--app-id", required=True, help="飞书应用 ID")
    parser.add_argument("--app-secret", required=True, help="飞书应用密钥")
    
    args = parser.parse_args()
    setup_bot(args.app_id, args.app_secret)
