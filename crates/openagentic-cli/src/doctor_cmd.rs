//! 健康检查命令

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// 检查项结果
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckStatus {
    Ok,
    Warning,
    Error,
}

/// 运行健康检查
pub async fn run(fix: bool, verbose: bool) -> Result<()> {
    println!("\n🏥 OpenAgentic 系统健康检查\n");

    let mut results = Vec::new();

    // 运行所有检查
    results.push(check_rust_version());
    results.push(check_cargo());
    results.push(check_config_file());
    results.push(check_api_keys());
    results.push(check_dependencies());
    results.push(check_docker());
    results.push(check_podman());
    results.push(check_wasmtime());
    results.push(check_audio_devices());
    results.push(check_chrome());
    results.push(check_ports());

    // 显示结果
    let mut ok_count = 0;
    let mut warn_count = 0;
    let mut error_count = 0;

    for result in &results {
        let icon = match result.status {
            CheckStatus::Ok => "✅",
            CheckStatus::Warning => "⚠️",
            CheckStatus::Error => "❌",
        };

        println!("{} {}: {}", icon, result.name, result.message);

        if verbose && result.fix_hint.is_some() {
            println!("   💡 提示: {}", result.fix_hint.as_ref().unwrap());
        }

        match result.status {
            CheckStatus::Ok => ok_count += 1,
            CheckStatus::Warning => warn_count += 1,
            CheckStatus::Error => error_count += 1,
        }
    }

    // 总结
    println!("\n{}", "─".repeat(50));
    println!(
        "检查完成: {} 通过, {} 警告, {} 错误\n",
        ok_count, warn_count, error_count
    );

    // 自动修复
    if fix && (warn_count > 0 || error_count > 0) {
        println!("🔧 尝试自动修复问题...\n");
        run_fixes(&results)?;
    }

    // 根据结果给出建议
    if error_count > 0 {
        println!("❌ 发现错误，请先解决上述问题后再运行服务。");
        if !fix {
            println!("   运行 `open-agentic doctor --fix` 尝试自动修复。");
        }
    } else if warn_count > 0 {
        println!("⚠️  存在警告，服务可以运行但建议处理这些问题。");
    } else {
        println!("✅ 系统状态良好！可以运行 `open-agentic gateway` 启动服务。");
    }

    Ok(())
}

/// 检查 Rust 版本
fn check_rust_version() -> CheckResult {
    let output = Command::new("rustc").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            CheckResult {
                name: "Rust 版本".to_string(),
                status: CheckStatus::Ok,
                message: version.trim().to_string(),
                fix_hint: None,
            }
        }
        _ => CheckResult {
            name: "Rust 版本".to_string(),
            status: CheckStatus::Error,
            message: "未安装 Rust".to_string(),
            fix_hint: Some(
                "运行 `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` 安装 Rust"
                    .to_string(),
            ),
        },
    }
}

/// 检查 Cargo
fn check_cargo() -> CheckResult {
    let output = Command::new("cargo").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            CheckResult {
                name: "Cargo".to_string(),
                status: CheckStatus::Ok,
                message: version.trim().to_string(),
                fix_hint: None,
            }
        }
        _ => CheckResult {
            name: "Cargo".to_string(),
            status: CheckStatus::Error,
            message: "未找到 Cargo".to_string(),
            fix_hint: Some("Cargo 应该随 Rust 一起安装".to_string()),
        },
    }
}

/// 检查配置文件
fn check_config_file() -> CheckResult {
    let config_path = dirs::home_dir().map(|h| h.join(".open-agentic").join("openagentic.json"));

    match config_path {
        Some(path) if path.exists() => CheckResult {
            name: "配置文件".to_string(),
            status: CheckStatus::Ok,
            message: format!("存在于 {}", path.display()),
            fix_hint: None,
        },
        Some(path) => CheckResult {
            name: "配置文件".to_string(),
            status: CheckStatus::Warning,
            message: "配置文件不存在".to_string(),
            fix_hint: Some(format!(
                "运行 `open-agentic wizard` 创建配置，或创建 {}",
                path.display()
            )),
        },
        None => CheckResult {
            name: "配置文件".to_string(),
            status: CheckStatus::Error,
            message: "无法确定配置路径".to_string(),
            fix_hint: None,
        },
    }
}

/// 检查 API 密钥
fn check_api_keys() -> CheckResult {
    let mut missing_keys = Vec::new();
    let required_vars = ["OPENAI_API_KEY", "ANTHROPIC_API_KEY"];

    for var in required_vars {
        if std::env::var(var).is_err() {
            missing_keys.push(var);
        }
    }

    if missing_keys.is_empty() {
        CheckResult {
            name: "API 密钥".to_string(),
            status: CheckStatus::Ok,
            message: "已设置".to_string(),
            fix_hint: None,
        }
    } else {
        CheckResult {
            name: "API 密钥".to_string(),
            status: CheckStatus::Warning,
            message: format!("缺少: {}", missing_keys.join(", ")),
            fix_hint: Some(
                "在 ~/.open-agentic/openagentic.json 中设置 API 密钥，或设置环境变量".to_string(),
            ),
        }
    }
}

/// 检查项目依赖
fn check_dependencies() -> CheckResult {
    let cargo_lock = PathBuf::from("Cargo.lock");

    if cargo_lock.exists() {
        CheckResult {
            name: "项目依赖".to_string(),
            status: CheckStatus::Ok,
            message: "已安装".to_string(),
            fix_hint: None,
        }
    } else {
        CheckResult {
            name: "项目依赖".to_string(),
            status: CheckStatus::Warning,
            message: "未找到 Cargo.lock".to_string(),
            fix_hint: Some("运行 `cargo build` 安装依赖".to_string()),
        }
    }
}

/// 检查 Docker
fn check_docker() -> CheckResult {
    let output = Command::new("docker").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);

            // 检查 Docker 是否运行
            let running = Command::new("docker")
                .args(["info"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if running {
                CheckResult {
                    name: "Docker".to_string(),
                    status: CheckStatus::Ok,
                    message: version.trim().to_string(),
                    fix_hint: None,
                }
            } else {
                CheckResult {
                    name: "Docker".to_string(),
                    status: CheckStatus::Warning,
                    message: "已安装但未运行".to_string(),
                    fix_hint: Some("运行 `dockerd` 或启动 Docker Desktop".to_string()),
                }
            }
        }
        _ => CheckResult {
            name: "Docker".to_string(),
            status: CheckStatus::Warning,
            message: "未安装 (可选，用于沙箱功能)".to_string(),
            fix_hint: Some("访问 https://docs.docker.com/get-docker/ 安装 Docker".to_string()),
        },
    }
}

/// 检查 Podman
fn check_podman() -> CheckResult {
    let output = Command::new("podman").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);

            let running = Command::new("podman")
                .args(["info"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if running {
                CheckResult {
                    name: "Podman".to_string(),
                    status: CheckStatus::Ok,
                    message: version.trim().to_string(),
                    fix_hint: None,
                }
            } else {
                CheckResult {
                    name: "Podman".to_string(),
                    status: CheckStatus::Warning,
                    message: "已安装但未运行".to_string(),
                    fix_hint: Some("运行 `podman machine start`".to_string()),
                }
            }
        }
        _ => CheckResult {
            name: "Podman".to_string(),
            status: CheckStatus::Ok,
            message: "未安装 (可选，Docker 优先)".to_string(),
            fix_hint: None,
        },
    }
}

/// 检查 WASM Runtime (wasmtime)
fn check_wasmtime() -> CheckResult {
    let output = Command::new("wasmtime").arg("--version").output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            CheckResult {
                name: "WASM Runtime (wasmtime)".to_string(),
                status: CheckStatus::Ok,
                message: version.trim().to_string(),
                fix_hint: None,
            }
        }
        _ => CheckResult {
            name: "WASM Runtime (wasmtime)".to_string(),
            status: CheckStatus::Warning,
            message: "未安装 (可选，用于 WASM 沙箱)".to_string(),
            fix_hint: Some("运行 `curl https://wasmtime.dev/install.sh -sSf | bash` 安装".to_string()),
        },
    }
}

/// 检查音频设备
fn check_audio_devices() -> CheckResult {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("system_profiler")
            .args(["SPAudioDataType"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let info = String::from_utf8_lossy(&output.stdout);
                if info.contains("No audio devices found") || info.is_empty() {
                    return CheckResult {
                        name: "音频设备".to_string(),
                        status: CheckStatus::Warning,
                        message: "未检测到音频设备".to_string(),
                        fix_hint: Some("检查麦克风/扬声器连接".to_string()),
                    };
                } else {
                    return CheckResult {
                        name: "音频设备".to_string(),
                        status: CheckStatus::Ok,
                        message: "已检测到音频设备".to_string(),
                        fix_hint: None,
                    };
                }
            }
            _ => {
                return CheckResult {
                    name: "音频设备".to_string(),
                    status: CheckStatus::Warning,
                    message: "无法检测音频设备".to_string(),
                    fix_hint: None,
                };
            },
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("pactl")
            .args(["list", "short", "sources"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let info = String::from_utf8_lossy(&output.stdout);
                if info.is_empty() {
                    return CheckResult {
                        name: "音频设备".to_string(),
                        status: CheckStatus::Warning,
                        message: "未检测到音频设备".to_string(),
                        fix_hint: Some("检查麦克风连接并安装 pulseaudio".to_string()),
                    };
                } else {
                    return CheckResult {
                        name: "音频设备".to_string(),
                        status: CheckStatus::Ok,
                        message: "已检测到音频设备".to_string(),
                        fix_hint: None,
                    };
                }
            }
            _ => {
                return CheckResult {
                    name: "音频设备".to_string(),
                    status: CheckStatus::Warning,
                    message: "无法检测音频设备".to_string(),
                    fix_hint: None,
                };
            },
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        CheckResult {
            name: "音频设备".to_string(),
            status: CheckStatus::Ok,
            message: "跳过检测 (不支持的平台)".to_string(),
            fix_hint: None,
        }
    }
}

/// 检查 Chrome/Chromium
fn check_chrome() -> CheckResult {
    // macOS
    let chrome_paths = vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
    ];

    for path in chrome_paths {
        if PathBuf::from(path).exists() {
            return CheckResult {
                name: "Chrome/Chromium".to_string(),
                status: CheckStatus::Ok,
                message: format!("已安装 ({})", path),
                fix_hint: None,
            };
        }
    }

    CheckResult {
        name: "Chrome/Chromium".to_string(),
        status: CheckStatus::Warning,
        message: "未找到 (可选，用于浏览器控制功能)".to_string(),
        fix_hint: Some("安装 Google Chrome 或 Chromium".to_string()),
    }
}

/// 检查端口
fn check_ports() -> CheckResult {
    let ports = [18789, 8080, 3000];
    let mut conflicts = Vec::new();

    for port in ports {
        if is_port_in_use(port) {
            conflicts.push(port);
        }
    }

    if conflicts.is_empty() {
        CheckResult {
            name: "端口状态".to_string(),
            status: CheckStatus::Ok,
            message: "所需端口可用".to_string(),
            fix_hint: None,
        }
    } else {
        CheckResult {
            name: "端口状态".to_string(),
            status: CheckStatus::Warning,
            message: format!("端口已被占用: {:?}", conflicts),
            fix_hint: Some("停止占用端口的进程或修改配置中的端口".to_string()),
        }
    }
}

/// 检查端口是否被占用
fn is_port_in_use(port: u16) -> bool {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    TcpListener::bind(addr).is_err()
}

/// 运行自动修复
fn run_fixes(results: &[CheckResult]) -> Result<()> {
    for result in results {
        if result.status == CheckStatus::Error || result.status == CheckStatus::Warning {
            // 自动修复逻辑
            match result.name.as_str() {
                "配置文件" => {
                    // 创建默认配置
                    if let Some(home) = dirs::home_dir() {
                        let config_dir = home.join(".open-agentic");
                        std::fs::create_dir_all(&config_dir)?;
                        let config_path = config_dir.join("openagentic.json");

                        let default_config = serde_json::json!({
                            "user_name": "User",
                            "default_provider": "openai",
                            "default_model": "gpt-4o",
                        });

                        std::fs::write(
                            &config_path,
                            serde_json::to_string_pretty(&default_config)?,
                        )?;
                        println!("✅ 已创建默认配置文件: {}", config_path.display());
                    }
                }
                "项目依赖" => {
                    println!("📦 正在安装依赖...");
                    let _ = Command::new("cargo").args(["build"]).status();
                }
                _ => {}
            }
        }
    }

    println!();
    Ok(())
}
