use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;

static TRANSLATIONS: Lazy<RwLock<HashMap<String, HashMap<String, String>>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    let mut zh = HashMap::new();
    zh.insert("welcome".into(), "欢迎使用 OpenAgentic".into());
    zh.insert("error.not_found".into(), "资源未找到".into());
    zh.insert("error.timeout".into(), "请求超时".into());
    zh.insert("error.invalid_input".into(), "输入无效".into());
    zh.insert("error.internal".into(), "内部错误".into());
    zh.insert("search.placeholder".into(), "搜索...".into());
    zh.insert("search.no_results".into(), "未找到结果".into());
    zh.insert("search.loading".into(), "搜索中...".into());
    zh.insert("tool.web_search".into(), "网页搜索工具".into());
    zh.insert("tool.image_gen".into(), "图像生成工具".into());
    zh.insert("tool.filesystem".into(), "文件系统工具".into());
    zh.insert("tool.calculator".into(), "计算器工具".into());
    zh.insert("status.connected".into(), "已连接".into());
    zh.insert("status.disconnected".into(), "已断开".into());
    zh.insert("status.error".into(), "错误".into());
    zh.insert("action.submit".into(), "提交".into());
    zh.insert("action.cancel".into(), "取消".into());
    zh.insert("action.confirm".into(), "确认".into());
    zh.insert("action.retry".into(), "重试".into());
    map.insert("zh".into(), zh);
    
    let mut en = HashMap::new();
    en.insert("welcome".into(), "Welcome to OpenAgentic".into());
    en.insert("error.not_found".into(), "Resource not found".into());
    en.insert("error.timeout".into(), "Request timeout".into());
    en.insert("error.invalid_input".into(), "Invalid input".into());
    en.insert("error.internal".into(), "Internal error".into());
    en.insert("search.placeholder".into(), "Search...".into());
    en.insert("search.no_results".into(), "No results found".into());
    en.insert("search.loading".into(), "Searching...".into());
    en.insert("tool.web_search".into(), "Web Search Tool".into());
    en.insert("tool.image_gen".into(), "Image Generation Tool".into());
    en.insert("tool.filesystem".into(), "Filesystem Tool".into());
    en.insert("tool.calculator".into(), "Calculator Tool".into());
    en.insert("status.connected".into(), "Connected".into());
    en.insert("status.disconnected".into(), "Disconnected".into());
    en.insert("status.error".into(), "Error".into());
    en.insert("action.submit".into(), "Submit".into());
    en.insert("action.cancel".into(), "Cancel".into());
    en.insert("action.confirm".into(), "Confirm".into());
    en.insert("action.retry".into(), "Retry".into());
    map.insert("en".into(), en);
    
    RwLock::new(map)
});

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Locale {
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

impl Default for Locale {
    fn default() -> Self {
        Locale::English
    }
}

impl Locale {
    pub fn code(&self) -> &str {
        match self {
            Locale::Chinese => "zh",
            Locale::English => "en",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh" | "zh-cn" | "zh-tw" | "chinese" => Locale::Chinese,
            _ => Locale::English,
        }
    }
}

pub struct I18n;

impl I18n {
    pub fn t(locale: &Locale, key: &str) -> String {
        let code = locale.code();
        if let Ok(translations) = TRANSLATIONS.read() {
            if let Some(lang_map) = translations.get(code) {
                if let Some(value) = lang_map.get(key) {
                    return value.clone();
                }
            }
        }
        key.to_string()
    }
    
    pub fn available_locales() -> Vec<Locale> {
        vec![Locale::English, Locale::Chinese]
    }
    
    pub fn all_translations(locale: &Locale) -> HashMap<String, String> {
        let code = locale.code();
        if let Ok(translations) = TRANSLATIONS.read() {
            if let Some(lang_map) = translations.get(code) {
                return lang_map.clone();
            }
        }
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_default() {
        assert_eq!(Locale::default(), Locale::English);
    }

    #[test]
    fn test_locale_code() {
        assert_eq!(Locale::English.code(), "en");
        assert_eq!(Locale::Chinese.code(), "zh");
    }

    #[test]
    fn test_locale_from_str() {
        assert_eq!(Locale::from_str("zh"), Locale::Chinese);
        assert_eq!(Locale::from_str("zh-cn"), Locale::Chinese);
        assert_eq!(Locale::from_str("en"), Locale::English);
        assert_eq!(Locale::from_str("unknown"), Locale::English);
    }

    #[test]
    fn test_i18n_english() {
        let locale = Locale::English;
        assert_eq!(I18n::t(&locale, "welcome"), "Welcome to OpenAgentic");
        assert_eq!(I18n::t(&locale, "error.not_found"), "Resource not found");
    }

    #[test]
    fn test_i18n_chinese() {
        let locale = Locale::Chinese;
        assert_eq!(I18n::t(&locale, "welcome"), "欢迎使用 OpenAgentic");
        assert_eq!(I18n::t(&locale, "error.not_found"), "资源未找到");
    }

    #[test]
    fn test_i18n_missing_key() {
        let locale = Locale::English;
        assert_eq!(I18n::t(&locale, "nonexistent.key"), "nonexistent.key");
    }

    #[test]
    fn test_available_locales() {
        let locales = I18n::available_locales();
        assert_eq!(locales.len(), 2);
        assert!(locales.contains(&Locale::English));
        assert!(locales.contains(&Locale::Chinese));
    }

    #[test]
    fn test_all_translations() {
        let locale = Locale::English;
        let translations = I18n::all_translations(&locale);
        assert!(!translations.is_empty());
        assert_eq!(translations.get("welcome"), Some(&"Welcome to OpenAgentic".to_string()));
    }
}
