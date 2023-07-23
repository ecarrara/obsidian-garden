use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct Metadata {
    inner: HashMap<String, MetadataValue>,
}

impl Metadata {
    pub fn tags(&self) -> Vec<String> {
        let mut tags = Vec::new();

        if let Some(MetadataValue::String(tag)) = self.inner.get("tag") {
            for tag in tag.split(',').map(|t| t.trim()) {
                tags.push(tag.to_string());
            }
        }

        if let Some(MetadataValue::List(tag_list)) = self.inner.get("tags") {
            for tag in tag_list {
                if let MetadataValue::String(tag) = tag {
                    tags.push(tag.to_string());
                }
            }
        }

        tags
    }
}

impl From<HashMap<String, MetadataValue>> for Metadata {
    fn from(value: HashMap<String, MetadataValue>) -> Self {
        Self { inner: value }
    }
}

#[derive(Debug, PartialEq)]
pub enum MetadataValue {
    Boolean(bool),
    List(Vec<MetadataValue>),
    Map(HashMap<String, MetadataValue>),
    Null,
    Number(f64),
    String(String),
}

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("mapping key is not a string")]
    MetadataMappingKeyTypeError(),

    #[error("invalid YAML in frontmatter section")]
    FrontMatterYamlError(#[from] serde_yaml::Error),
}

impl TryFrom<serde_yaml::Value> for MetadataValue {
    type Error = MetadataError;

    fn try_from(value: serde_yaml::Value) -> Result<Self, Self::Error> {
        match value {
            serde_yaml::Value::Null => Ok(MetadataValue::Null),
            serde_yaml::Value::Bool(value) => Ok(MetadataValue::Boolean(value)),
            serde_yaml::Value::Number(value) => Ok(MetadataValue::Number(value.as_f64().unwrap())),
            serde_yaml::Value::String(value) => Ok(MetadataValue::String(value)),
            serde_yaml::Value::Sequence(values) => {
                let mut items = Vec::with_capacity(values.len());
                for value in values.into_iter() {
                    items.push(value.try_into()?);
                }
                Ok(MetadataValue::List(items))
            }
            serde_yaml::Value::Mapping(mapping) => {
                let mut items = HashMap::with_capacity(mapping.len());
                for (key, value) in mapping.into_iter() {
                    items.insert(
                        key.as_str()
                            .ok_or(MetadataError::MetadataMappingKeyTypeError())?
                            .to_string(),
                        value.try_into()?,
                    );
                }

                Ok(MetadataValue::Map(items))
            }
            serde_yaml::Value::Tagged(_) => todo!(),
        }
    }
}

pub fn parse_frontmatter(content: &str) -> Result<(Metadata, &str), MetadataError> {
    let mut metadata = HashMap::new();

    const MARKER: &str = "---\n";

    if let Some(content) = content.strip_prefix(MARKER) {
        if let Some(pos) = content.find(MARKER) {
            let value: serde_yaml::Value = serde_yaml::from_str(&content[..pos])?;
            let mapping = value
                .as_mapping()
                .ok_or(MetadataError::MetadataMappingKeyTypeError())?;

            for (key, value) in mapping.into_iter() {
                let key = key
                    .as_str()
                    .ok_or(MetadataError::MetadataMappingKeyTypeError())?
                    .to_string();

                let metadata_value = MetadataValue::try_from(value.clone())?;

                metadata.insert(key, metadata_value);
            }

            Ok((Metadata::from(metadata), &content[pos + MARKER.len()..]))
        } else {
            Ok((Metadata::from(metadata), content))
        }
    } else {
        Ok((Metadata::from(metadata), content))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_frontmatter;
    use crate::metadata::{Metadata, MetadataError, MetadataValue};
    use std::collections::HashMap;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
key1: value1
tags:
  - t1
  - t2
---
"#;
        let mut metadata = HashMap::new();
        metadata.insert(
            "key1".to_string(),
            MetadataValue::String("value1".to_string()),
        );
        metadata.insert(
            "tags".to_string(),
            MetadataValue::List(vec![
                MetadataValue::String("t1".to_string()),
                MetadataValue::String("t2".to_string()),
            ]),
        );
        assert_eq!(
            parse_frontmatter(content).unwrap(),
            (Metadata::from(metadata), "")
        );
    }

    #[test]
    fn test_parse_frontmatter_invalid_yaml() {
        let content = r#"---
key1: value1
key1: value1
---
"#;
        assert!(matches!(
            parse_frontmatter(content).unwrap_err(),
            MetadataError::FrontMatterYamlError(_)
        ),);
    }

    #[test]
    fn test_parse_frontmatter_mapping_key_is_not_string() {
        let content = r#"---
42: oops
---
"#;
        assert!(matches!(
            parse_frontmatter(content).unwrap_err(),
            MetadataError::MetadataMappingKeyTypeError()
        ),);
    }
}
