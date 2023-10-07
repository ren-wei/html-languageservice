use std::collections::HashMap;

use crate::html_data::{HTMLDataV1, IAttributeData, ITagData, IValueData};

pub struct HTMLDataProvider {
    id: String,
    tags: Vec<ITagData>,
    tag_map: HashMap<String, usize>,
    global_attributes: Vec<IAttributeData>,
    value_set_map: HashMap<String, Vec<IValueData>>,
}

pub trait IHTMLDataProvider {
    fn get_id(&self) -> &str;
    fn is_applicable(&self, language_id: &str) -> bool;
    fn provide_tags(&self) -> &Vec<ITagData>;
    fn provide_attributes(&self, tag: &str) -> Vec<&IAttributeData>;
    fn provide_values(&self, tag: &str, attribute: &str) -> Vec<&IValueData>;
}

impl HTMLDataProvider {
    pub fn new(id: String, custom_data: HTMLDataV1) -> HTMLDataProvider {
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

    fn provide_attributes(&self, tag: &str) -> Vec<&IAttributeData> {
        let mut attributes = vec![];

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

        let attribute = attribute.to_lowercase();

        let tag_entry = self.tag_map.get(tag);
        if let Some(tag_entry_index) = tag_entry {
            let tag_entry = &self.tags[*tag_entry_index];
            for a in &tag_entry.attributes {
                if a.name.to_lowercase() == attribute {
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
            if a.name.to_lowercase() == attribute {
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
