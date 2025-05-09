use std::collections::HashMap;

use lsp_textdocument::FullTextDocument;
use lsp_types::{MarkupContent, MarkupKind};

use crate::{
    html_data::{Description, HTMLDataV1, IAttributeData, IReference, ITagData, IValueData},
    parser::html_document::HTMLDocument,
    utils::markup,
};

/// Built-in data provider that provides information for `HTMLDataManager`
pub struct HTMLDataProvider {
    id: String,
    tags: Vec<ITagData>,
    tag_map: HashMap<String, usize>,
    global_attributes: Vec<IAttributeData>,
    value_set_map: HashMap<String, Vec<IValueData>>,
    case_sensitive: bool,
}

/// To implement that the data provider can provide information to the `HTMLDataManager`
pub trait IHTMLDataProvider: Send + Sync {
    /// The ID of the data provider, which cannot be duplicated,
    /// note that the ID of the built-in data provider is "html5"
    fn get_id(&self) -> &str;
    fn is_applicable(&self, language_id: &str) -> bool;
    fn provide_tags(&self) -> &Vec<ITagData>;
    fn provide_attributes(
        &self,
        tag: &str,
        content: &HTMLDataProviderContent<'_>,
    ) -> Vec<&IAttributeData>;
    fn provide_values(&self, tag: &str, attribute: &str) -> Vec<&IValueData>;
}

pub struct HTMLDataProviderContent<'a> {
    pub document: &'a FullTextDocument,
    pub html_document: &'a HTMLDocument,
    pub offset: usize,
}

impl HTMLDataProvider {
    pub fn new(id: String, custom_data: HTMLDataV1, case_sensitive: bool) -> HTMLDataProvider {
        let mut tag_map = HashMap::new();
        if let Some(tags) = &custom_data.tags {
            for (i, tag) in tags.iter().enumerate() {
                tag_map.insert(tag.name.clone(), i);
            }
        }

        let mut value_set_map = HashMap::new();

        if let Some(value_sets) = custom_data.value_sets {
            for vs in value_sets {
                value_set_map.insert(vs.name, vs.values);
            }
        }

        HTMLDataProvider {
            id,
            tags: custom_data.tags.unwrap_or_default(),
            tag_map,
            global_attributes: custom_data.global_attributes.unwrap_or_default(),
            value_set_map,
            case_sensitive,
        }
    }
}

impl IHTMLDataProvider for HTMLDataProvider {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn is_applicable(&self, _language_id: &str) -> bool {
        true
    }

    fn provide_tags(&self) -> &Vec<ITagData> {
        &self.tags
    }

    fn provide_attributes(
        &self,
        tag: &str,
        _content: &HTMLDataProviderContent,
    ) -> Vec<&IAttributeData> {
        let mut attributes = vec![];

        let tag = if self.case_sensitive {
            tag
        } else {
            &tag.to_lowercase()
        };
        let tag_entry_index = self.tag_map.get(tag);
        if let Some(tag_entry_index) = tag_entry_index {
            let tag_entry = &self.tags[*tag_entry_index];
            for attribute in &tag_entry.attributes {
                attributes.push(attribute);
            }
        }
        for attribute in &self.global_attributes {
            attributes.push(&attribute);
        }

        attributes
    }

    fn provide_values(&self, tag: &str, attribute: &str) -> Vec<&IValueData> {
        let mut values = vec![];

        let attribute = if self.case_sensitive {
            attribute
        } else {
            &attribute.to_lowercase()
        };

        let tag = if self.case_sensitive {
            tag
        } else {
            &tag.to_lowercase()
        };
        let tag_entry = self.tag_map.get(tag);
        if let Some(tag_entry_index) = tag_entry {
            let tag_entry = &self.tags[*tag_entry_index];
            for a in &tag_entry.attributes {
                let equal = if self.case_sensitive {
                    a.name == attribute
                } else {
                    a.name.to_lowercase() == attribute
                };
                if equal {
                    if let Some(a_values) = &a.values {
                        for value in a_values {
                            values.push(value);
                        }
                    }
                    if let Some(value_set) = &a.value_set {
                        if let Some(set) = &self.value_set_map.get(value_set) {
                            for v in *set {
                                values.push(v);
                            }
                        }
                    }
                }
            }
        }
        for a in &self.global_attributes {
            let equal = if self.case_sensitive {
                a.name == attribute
            } else {
                a.name.to_lowercase() == attribute
            };
            if equal {
                if let Some(a_values) = &a.values {
                    for value in a_values {
                        values.push(value);
                    }
                }
                if let Some(value_set) = &a.value_set {
                    if let Some(set) = &self.value_set_map.get(value_set) {
                        for v in *set {
                            values.push(v);
                        }
                    }
                }
            }
        }

        values
    }
}

/// Generate Documentation used in hover/complete From documentation and references
pub fn generate_documentation(
    item: GenerateDocumentationItem,
    setting: GenerateDocumentationSetting,
) -> Option<MarkupContent> {
    let mut result = MarkupContent {
        kind: if setting.does_support_markdown {
            MarkupKind::Markdown
        } else {
            MarkupKind::PlainText
        },
        value: String::new(),
    };

    if item.description.is_some() && setting.documentation {
        let normalized_description = markup::normalize_markup_content(item.description.unwrap());
        result.value += &normalized_description.value;
    }

    if item.references.as_deref().is_some_and(|r| r.len() > 0) && setting.references {
        if result.value.len() > 0 {
            result.value += "\n\n";
        }
        let references = item.references.unwrap();
        if setting.does_support_markdown {
            result.value += &references
                .iter()
                .map(|r| format!("[{}]({})", r.name, r.url))
                .collect::<Vec<String>>()
                .join(" | ");
        } else {
            result.value += &references
                .iter()
                .map(|r| format!("{}: {}", r.name, r.url))
                .collect::<Vec<String>>()
                .join("\n");
        }
    }

    if result.value.len() > 0 {
        Some(result)
    } else {
        None
    }
}

pub struct GenerateDocumentationItem {
    pub description: Option<Description>,
    pub references: Option<Vec<IReference>>,
}

pub struct GenerateDocumentationSetting {
    pub documentation: bool,
    pub references: bool,
    pub does_support_markdown: bool,
}
