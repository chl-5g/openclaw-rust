#!/usr/bin/env python3
"""
软件工作流初始化脚本
"""

import argparse
import json
import yaml
from pathlib import Path

def init_workflow(workflow_name: str):
    """初始化工作流"""
    
    # 创建默认配置
    config = {
        "name": workflow_name,
        "stages": [
            {
                "name": "planning",
                "agent": "ceo",
                "description": "规划阶段"
            },
            {
                "name": "analysis", 
                "agent": "product_manager",
                "description": "需求分析阶段"
            },
            {
                "name": "development",
                "agent": "engineer",
                "description": "开发阶段"
            },
            {
                "name": "testing",
                "agent": "qa",
                "description": "测试阶段"
            },
            {
                "name": "review",
                "agent": "ceo",
                "description": "审核阶段"
            }
        ],
        "initial_stage": "planning"
    }
    
    config_path = Path("config")
    config_path.mkdir(exist_ok=True)
    
    output_file = config_path / "stages.yaml"
    with open(output_file, "w", encoding="utf-8") as f:
        yaml.dump(config, f, allow_unicode=True)
    
    print(f"✅ 工作流初始化完成: {output_file}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="软件工作流初始化")
    parser.add_argument("--workflow", default="software_dev", help="工作流名称")
    
    args = parser.parse_args()
    init_workflow(args.workflow)
