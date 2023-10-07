use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HTMLDataV1 {
    pub version: f32,
    pub tags: Option<Vec<ITagData>>,
    #[serde(rename = "globalAttributes")]
    pub global_attributes: Option<Vec<IAttributeData>>,
    #[serde(rename = "valueSets")]
    pub value_sets: Option<Vec<IValueSet>>,
}

#[derive(Serialize, Deserialize)]
pub struct ITagData {
    pub name: String,
    pub description: Option<Description>,
    pub attributes: Vec<IAttributeData>,
    pub references: Option<Vec<IReference>>,
    pub void: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct IAttributeData {
    pub name: String,
    pub description: Option<Description>,
    #[serde(rename = "valueSet")]
    pub value_set: Option<String>,
    pub values: Option<Vec<IValueData>>,
    pub references: Option<Vec<IReference>>,
}

#[derive(Serialize, Deserialize)]
pub struct IValueSet {
    pub name: String,
    pub values: Vec<IValueData>,
}

#[derive(Serialize, Deserialize)]
pub struct IValueData {
    pub name: String,
    pub description: Option<Description>,
    pub references: Option<Vec<IReference>>,
}

#[derive(Serialize, Deserialize)]
pub struct IReference {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Description {
    String(String),
    MarkupContent(MarkupContent),
}

#[derive(Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub enum MarkupKind {
    #[serde(rename = "plaintext")]
    Plaintext,
    #[serde(rename = "markdown")]
    Markdown,
}
