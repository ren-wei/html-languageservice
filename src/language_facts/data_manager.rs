use std::sync::Arc;

use lazy_static::lazy_static;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use super::{
    data_provider::{HTMLDataProvider, IHTMLDataProvider},
    web_custom_data::HTML_DATA,
};

pub struct HTMLDataManager {
    data_providers: Vec<Arc<RwLock<dyn IHTMLDataProvider>>>,
}

impl HTMLDataManager {
    pub fn new(
        use_default_data_provider: bool,
        custom_data_providers: Option<Vec<Arc<RwLock<dyn IHTMLDataProvider>>>>,
    ) -> HTMLDataManager {
        let mut data_manager = HTMLDataManager {
            data_providers: vec![],
        };
        data_manager.set_data_providers(
            use_default_data_provider,
            custom_data_providers.unwrap_or(vec![]),
        );
        data_manager
    }

    pub fn set_data_providers(
        &mut self,
        built_in: bool,
        mut providers: Vec<Arc<RwLock<dyn IHTMLDataProvider>>>,
    ) {
        self.data_providers.clear();
        if built_in {
            let data = serde_json::from_str(HTML_DATA).unwrap();
            self.data_providers
                .push(Arc::new(RwLock::new(HTMLDataProvider::new(
                    "html5".to_string(),
                    data,
                ))));
        }
        self.data_providers.append(&mut providers);
    }

    pub fn get_data_providers(&self) -> &Vec<Arc<RwLock<dyn IHTMLDataProvider>>> {
        &self.data_providers
    }

    pub fn is_void_element(&self, e: &str, void_elements: &Vec<String>) -> bool {
        void_elements.contains(&e.to_string())
    }

    pub async fn get_void_elements(&self, language_id: &str) -> Vec<String> {
        let mut void_tags: Vec<String> = vec![];
        for provider in &self.data_providers {
            if provider.read().await.is_applicable(language_id) {
                provider
                    .read()
                    .await
                    .provide_tags()
                    .iter()
                    .filter(|tag| tag.void.is_some_and(|v| v))
                    .for_each(|tag| void_tags.push(tag.name.clone()))
            }
        }
        void_tags.sort();
        void_tags
    }

    pub fn is_path_attribute(&self, tag: &str, attr: &str) -> bool {
        if ["src", "href"].contains(&attr) {
            return true;
        }
        let value = PATH_TAG_AND_ATTR.as_object().unwrap().get(tag);
        if let Some(value) = value {
            if value.is_array() {
                value
                    .as_array()
                    .unwrap()
                    .contains(&Value::String(attr.to_string()))
            } else {
                value.as_str().unwrap() == attr
            }
        } else {
            false
        }
    }
}

lazy_static! {
static ref PATH_TAG_AND_ATTR: Value = json!({
    // HTML 4
    "a": "href",
    "area": "href",
    "body": "background",
    "blockquote": "cite",
    "del": "cite",
    "form": "action",
    "frame": ["src", "longdesc"],
    "img": ["src", "longdesc"],
    "ins": "cite",
    "link": "href",
    "object": "data",
    "q": "cite",
    "script": "src",
    // HTML 5
    "audio": "src",
    "button": "formaction",
    "command": "icon",
    "embed": "src",
    "html": "manifest",
    "input": ["src", "formaction"],
    "source": "src",
    "track": "src",
    "video": ["src", "poster"]
});
}
