use jsonschema::JSONSchema;
use serde::{de::Visitor, Deserialize, Serialize};
use thiserror::*;

#[derive(Error, Debug)]
pub enum RuleError<'e> {
    #[error(transparent)]
    BuilderError(derive_builder::UninitializedFieldError),

    #[error(transparent)]
    InvalidSchema(jsonschema::ValidationError<'e>),
}

impl From<derive_builder::UninitializedFieldError> for RuleError<'_> {
    fn from(err: derive_builder::UninitializedFieldError) -> Self {
        Self::BuilderError(err)
    }
}

#[derive(Default, Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct RuleId(u128);

impl RuleId {
    pub fn next() -> Self {
        Self(uuid::Uuid::new_v4().to_u128_le())
    }
}

#[derive(Default, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct RuleName(String);

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    #[default]
    Problem,
    Suggestion,
    Layout,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum RuleFixability {
    #[default]
    Code,
    Whitespace,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Docs {
    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub recommended: bool,

    #[serde(default)]
    pub url: Option<url::Url>,
}

#[derive(Debug)]
pub struct Schema {
    raw: serde_json::Value,
    pub schema: JSONSchema,
}

impl Schema {
    pub fn new(raw: serde_json::Value, schema: JSONSchema) -> Self {
        Self { raw, schema }
    }

    pub fn compile(raw: &serde_json::Value) -> Result<JSONSchema, RuleError<'_>> {
        let schema = JSONSchema::compile(raw).map_err(RuleError::InvalidSchema)?;
        Ok(schema)
    }
}

impl Clone for Schema {
    fn clone(&self) -> Self {
        let raw = self.raw.clone();
        let schema = Schema::compile(&raw).unwrap();
        Schema::new(raw, schema)
    }
}

struct SchemaVisitor;
impl<'de> Visitor<'de> for SchemaVisitor {
    type Value = Schema;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid JSON Schema definition")
    }
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let schema = Schema::compile(&raw).map_err(serde::de::Error::custom)?;
        Ok(Schema::new(raw, schema))
    }
}

impl Serialize for Schema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.raw.serialize(serializer)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(rename = "type")]
    pub kind: RuleType,

    #[serde(default)]
    pub docs: Option<Docs>,

    #[serde(default)]
    pub fixable: Option<RuleFixability>,

    #[serde(default)]
    pub has_suggestions: bool,

    #[serde(default)]
    pub deprecated: bool,

    #[serde(default)]
    pub schema: Vec<Schema>,

    #[serde(default)]
    pub replaced_by: Vec<RuleName>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub name: String,
    pub meta: Meta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_void() {
        let no_void_js = include_str!("./rules/no-void.js");
        let rule: Rule = serde_json::from_str(no_void_js).unwrap();
        dbg!(&rule);
    }
}
