use lazy_static::lazy_static;
use serde_json::{json, Value};

use super::{
    data_provider::{HTMLDataProvider, IHTMLDataProvider},
    web_custom_data::HTML_DATA_INSTANCE,
};

/// Provides tags, attributes, and attribute value and so on,
/// for completion proposals and hover information.
/// It has standard data built-in and can be customized
pub struct HTMLDataManager {
    data_providers: Vec<Box<dyn IHTMLDataProvider>>,
    case_sensitive: bool,
}

impl HTMLDataManager {
    pub(crate) fn new(
        use_default_data_provider: bool,
        custom_data_providers: Option<Vec<Box<dyn IHTMLDataProvider>>>,
        case_sensitive: bool,
    ) -> HTMLDataManager {
        let mut data_manager = HTMLDataManager {
            data_providers: vec![],
            case_sensitive,
        };
        data_manager.set_data_providers(
            use_default_data_provider,
            custom_data_providers.unwrap_or(vec![]),
        );
        data_manager
    }

    /// Set up a data provider, and the old data will be cleaned
    pub fn set_data_providers(
        &mut self,
        built_in: bool,
        mut providers: Vec<Box<dyn IHTMLDataProvider>>,
    ) {
        self.data_providers.clear();
        if built_in {
            self.data_providers.push(Box::new(HTMLDataProvider::new(
                "html5".to_string(),
                HTML_DATA_INSTANCE.clone(),
                self.case_sensitive,
            )));
        }
        self.data_providers.append(&mut providers);
    }

    pub fn get_data_providers(&self) -> &Vec<Box<dyn IHTMLDataProvider>> {
        &self.data_providers
    }

    /// Is the tag void element
    ///
    /// `void_elements` is from `get_void_elements`, and you should cache it to avoid duplicate void_elements generation
    pub fn is_void_element(&self, tag: &str, void_elements: &Vec<String>) -> bool {
        void_elements.contains(&tag.to_string())
    }

    /// Get `void_elements` from data_provider and you should cache it if you make sure it doesn't change
    pub fn get_void_elements(&self, language_id: &str) -> Vec<String> {
        let mut void_tags: Vec<String> = vec![];
        for provider in &self.data_providers {
            if provider.is_applicable(language_id) {
                provider
                    .provide_tags()
                    .iter()
                    .filter(|tag| tag.void.is_some_and(|v| v))
                    .for_each(|tag| void_tags.push(tag.name.clone()))
            }
        }
        void_tags.sort();
        void_tags
    }

    /// Is the `attr` of `tag` a path attribute
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

impl Default for HTMLDataManager {
    fn default() -> Self {
        HTMLDataManager::new(true, None, false)
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
