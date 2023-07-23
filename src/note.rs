use pulldown_cmark::Event;
use std::path::Path;
use thiserror::Error;

use crate::{
    metadata::{parse_frontmatter, Metadata, MetadataError},
    wikilink::{Wikilink, WikilinkParser},
};

#[derive(Debug, PartialEq)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub links: Vec<Wikilink>,
    pub metadata: Metadata,
}

impl Note {
    pub fn parse(title: &str, content: &str) -> Result<Note, NoteError> {
        let (metadata, content) = parse_frontmatter(content)?;

        let parser = pulldown_cmark::Parser::new(content);

        let mut links = Vec::new();
        let mut tags = metadata.tags();

        let mut wikilink_parser = WikilinkParser::new();
        for event in parser {
            if let Event::Text(text) = event {
                if let Some(link) = wikilink_parser.feed(&text) {
                    links.push(link);
                }

                collect_tags(&text, &mut tags);
            }
        }

        Ok(Note {
            title: title.into(),
            content: content.into(),
            tags,
            links,
            metadata,
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: &P) -> Result<Note, NoteError> {
        let content = std::fs::read_to_string(path)?;
        Note::parse("example", &content)
    }
}

fn collect_tags(text: &str, tags: &mut Vec<String>) {
    let mut tag_start = 0;

    for (i, chr) in text.chars().enumerate() {
        if chr == '#' {
            tag_start = i + 1;
        } else if tag_start > 0
            && !(chr.is_alphanumeric() || chr == '_' || chr == '-' || chr == '/')
        {
            tags.push(text[tag_start..i].to_string());
            tag_start = 0;
        }
    }

    if tag_start > 0 {
        tags.push(text[tag_start..].to_string());
    }
}

#[derive(Error, Debug)]
pub enum NoteError {
    #[error("io error")]
    IOError(#[from] std::io::Error),

    // #[error("invalid frontmatter type")]
    // FrontMatterInvalidType(),
    #[error("frontmatter value error")]
    MetadataValueError(#[from] MetadataError),
}

#[cfg(test)]
mod tests {
    use crate::metadata::MetadataValue;

    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_note() {
        let content = include_str!("../notes/example.md");
        let note = Note::parse("Example", content).expect("note parse");

        let mut metadata = HashMap::new();
        metadata.insert("published".to_string(), MetadataValue::Boolean(true));
        metadata.insert(
            "category".to_string(),
            MetadataValue::String("Example".to_string()),
        );

        assert_eq!(
            note,
            Note {
                title: "Example".to_string(),
                content: r#"#example

Example content. With #test tag inside.

## Heading 2

[[Page Name|Link label]]

This is a [[WikiLink]]. And this is a [Markdown Link](https://example.com)

Inline `let a = 2 + 2;` example

#code/rust

```rust
fn main () {
    println!("ok");
}
```"#
                    .to_string(),
                tags: vec![
                    "example".to_string(),
                    "test".to_string(),
                    "code/rust".to_string()
                ],
                links: vec![
                    Wikilink::new("Page Name", Some("Link label")),
                    Wikilink::new("WikiLink", None),
                ],
                metadata: Metadata::from(metadata),
            }
        );
    }
}
