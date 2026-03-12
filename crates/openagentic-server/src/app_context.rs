//! 应用上下文模块

use std::sync::Arc;
use tokio::sync::RwLock;

use openagentic_ai::AIProvider;
use openagentic_core::Config;
use openagentic_device::UnifiedDeviceManager;
use openagentic_memory::factory::MemoryBackend;
use openagentic_security::pipeline::SecurityPipeline;
use openagentic_sandbox::SandboxManager;
use openagentic_tools::ToolRegistry;

use crate::device_manager::DeviceManager;
use crate::orchestrator::ServiceOrchestrator;
use crate::vector_store_registry::VectorStoreRegistry;
use crate::voice_service::VoiceService;
use crate::acp_service::AcpService;

#[derive(Clone)]
pub struct AppContext {
    pub config: Config,
    pub ai_provider: Arc<dyn AIProvider>,
    pub memory_backend: Option<Arc<dyn MemoryBackend>>,
    pub security_pipeline: Arc<SecurityPipeline>,
    pub tool_registry: Arc<ToolRegistry>,
    pub sandbox_manager: Option<Arc<SandboxManager>>,
    pub orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    pub device_manager: Option<Arc<DeviceManager>>,
    pub unified_device_manager: Option<Arc<UnifiedDeviceManager>>,
    pub voice_service: Arc<VoiceService>,
    pub vector_store_registry: Arc<VectorStoreRegistry>,
    pub acp_service: Option<Arc<AcpService>>,
}

impl AppContext {
    pub fn new(
        config: Config,
        ai_provider: Arc<dyn AIProvider>,
        memory_backend: Option<Arc<dyn MemoryBackend>>,
        security_pipeline: Arc<SecurityPipeline>,
        tool_registry: Arc<ToolRegistry>,
        sandbox_manager: Option<Arc<SandboxManager>>,
        orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
        device_manager: Option<Arc<DeviceManager>>,
        unified_device_manager: Option<Arc<UnifiedDeviceManager>>,
        voice_service: Arc<VoiceService>,
        vector_store_registry: Arc<VectorStoreRegistry>,
        acp_service: Option<Arc<AcpService>>,
    ) -> Self {
        Self {
            config,
            ai_provider,
            memory_backend,
            security_pipeline,
            tool_registry,
            sandbox_manager,
            orchestrator,
            device_manager,
            unified_device_manager,
            voice_service,
            vector_store_registry,
            acp_service,
        }
    }

    pub async fn get_agent(&self, name: &str) -> Option<Arc<dyn openagentic_agent::Agent>> {
        let orchestrator = self.orchestrator.read().await;
        orchestrator.as_ref()?.get_agent(name).await
    }
}
