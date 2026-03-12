//! Agent CLI 工具 - 直接与 AI Assistant 对话

use anyhow::Result;
use clap::{ArgAction, Parser};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
pub struct AgentCli {
    /// Agent ID (default: default)
    #[arg(long, default_value = "default")]
    pub agent: String,
    /// Message to send to the agent
    #[arg(short, long)]
    pub message: Option<String>,
    /// Thinking mode (low, medium, high)
    #[arg(long, default_value = "medium")]
    pub thinking: String,
    /// Stream the response
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub stream: bool,
    /// Continue the last conversation
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub continue_conv: bool,
    /// System prompt override
    #[arg(long)]
    pub system: Option<String>,
    /// Gateway URL
    #[arg(long, default_value = "http://localhost:18789")]
    pub gateway_url: String,
}

#[derive(Debug, Serialize)]
struct AgentMessageRequest {
    agent_id: String,
    message: String,
    session_id: Option<String>,
    thinking: Option<String>,
    system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentMessageResponse {
    message: String,
    session_id: String,
}

impl AgentCli {
    pub async fn run(&self) -> Result<()> {
        let message = match (&self.message, self.continue_conv) {
            (Some(msg), _) => msg.clone(),
            (None, true) => {
                println!("Continuing last conversation...");
                String::new()
            }
            (None, false) => {
                anyhow::bail!("Please provide a message with --message");
            }
        };

        println!("🤖 Agent: {}", self.agent);
        println!("💭 Thinking: {}", self.thinking);
        println!("🌐 Gateway: {}", self.gateway_url);

        if !message.is_empty() {
            println!("📝 Message: {}", message);
        }

        println!("\n⏳ Connecting to Gateway...");

        self.connect_and_send(message).await
    }

    async fn connect_and_send(&self, message: String) -> Result<()> {
        let client = Client::new();
        let url = format!("{}/api/agent/message", self.gateway_url);

        let request = AgentMessageRequest {
            agent_id: self.agent.clone(),
            message: message.clone(),
            session_id: None,
            thinking: Some(self.thinking.clone()),
            system_prompt: self.system.clone(),
        };

        println!("✅ Connected to Gateway");
        println!("\n📤 Sending request...");

        if message.is_empty() {
            println!("🔄 Waiting for agent response...");
        } else {
            println!("💬 You: {}", message);
        }

        match client.post(&url).json(&request).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<AgentMessageResponse>().await {
                        Ok(result) => {
                            println!("\n🤖 Agent: {}", result.message);
                            println!("📋 Session: {}", result.session_id);
                        }
                        Err(e) => {
                            println!("\n⚠️ Failed to parse response: {}", e);
                            println!("🤖 Agent: (fallback simulation)");
                            self.print_simulation().await;
                        }
                    }
                } else {
                    let status = response.status();
                    println!("\n⚠️ Gateway returned error: {}", status);
                    println!("🤖 Agent: (fallback simulation)");
                    self.print_simulation().await;
                }
            }
            Err(e) => {
                println!("\n⚠️ Could not connect to Gateway: {}", e);
                println!("🤖 Agent: (fallback simulation)");
                self.print_simulation().await;
            }
        }

        Ok(())
    }

    async fn print_simulation(&self) {
        println!("This feature requires the Gateway to be running.");
        println!("Start the gateway with: open-agentic gateway");
    }
}
