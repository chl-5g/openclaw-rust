//! OpenAgentic Browser - Puppeteer 风格的浏览器控制模块
//!
//! 提供浏览器控制接口（截图、页面操作等）

use std::sync::Arc;

pub mod browser;
pub mod page;
pub mod screenshot;
pub mod types;

pub use browser::{Browser, BrowserError, BrowserPool, BrowserInfo};
pub use page::Page;
pub use screenshot::{ScreenshotUtils, PdfUtils};

pub use types::{
    BrowserId, PageId, BrowserConfig, ProxyConfig, PageOptions, NavigationOptions,
    WaitUntil, ScreenshotOptions, ScreenshotFormat, ClipRect, ClickOptions, MouseButton,
    TypeOptions, ScrollOptions, ScrollDistance, Point, Selector, ElementInfo, BoundingBox,
    UploadOptions, FileData, PageState, Cookie, BrowserMetrics, PdfOptions, PaperFormat,
    PdfMargins, BrowserEvent, JsResult,
};

#[async_trait::async_trait]
pub trait BrowserClient: Send + Sync {
    fn browser_id(&self) -> &BrowserId;
    
    async fn new_page(&self) -> Result<PageId, BrowserError>;
    
    async fn get_page(&self, page_id: &PageId) -> Result<Option<Arc<Page>>, BrowserError>;
    
    async fn close_page(&self, page_id: &PageId) -> Result<(), BrowserError>;
    
    async fn get_pages(&self) -> Result<Vec<PageId>, BrowserError>;
    
    async fn close(&self) -> Result<(), BrowserError>;
}

#[async_trait::async_trait]
impl BrowserClient for Browser {
    fn browser_id(&self) -> &BrowserId {
        &self.id
    }
    
    async fn new_page(&self) -> Result<PageId, BrowserError> {
        let page = Browser::new_page(self).await?;
        Ok(page.id.clone())
    }
    
    async fn get_page(&self, page_id: &PageId) -> Result<Option<Arc<Page>>, BrowserError> {
        Ok(Browser::get_page(self, page_id).await)
    }
    
    async fn close_page(&self, page_id: &PageId) -> Result<(), BrowserError> {
        Browser::close_page(self, page_id).await
    }
    
    async fn get_pages(&self) -> Result<Vec<PageId>, BrowserError> {
        Ok(Browser::get_pages(self).await)
    }
    
    async fn close(&self) -> Result<(), BrowserError> {
        Browser::close(self).await
    }
}
