#!/usr/bin/env python3
"""
OpenClaw 配置校验脚本
"""

import argparse
import yaml
import os
from pathlib import Path

def validate_config(config_dir: str):
    """校验配置文件"""
    
    config_path = Path(config_dir)
    errors = []
    
    # 检查必需的配置文件
    required_files = ["agents.yaml", "channels.yaml", "bindings.yaml"]
    for fname in required_files:
        fpath = config_path / fname
        if not fpath.exists():
            errors.append(f"缺少必需配置文件: {fname}")
    
    if errors:
        for e in errors:
            print(f"❌ {e}")
        return False
    
    # 校验 agents.yaml
    try:
        with open(config_path / "agents.yaml") as f:
            agents = yaml.safe_load(f)
            if not agents.get("agents"):
                errors.append("agents.yaml 中没有定义任何 agent")
    except Exception as e:
        errors.append(f"agents.yaml 解析失败: {e}")
    
    if errors:
        print("配置校验失败:")
        for e in errors:
            print(f"  - {e}")
        return False
    
    print("✅ 配置校验通过")
    return True

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="OpenClaw 配置校验")
    parser.add_argument("--config-dir", default="./config", help="配置文件目录")
    
    args = parser.parse_args()
    validate_config(args.config_dir)
