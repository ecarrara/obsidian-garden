use pulldown_cmark::CowStr;
use serde::Serialize;
use std::fmt::Display;

#[derive(Clone, PartialEq, Debug, Default, Serialize)]
pub struct Wikilink {
    pub target: String,
    pub label: Option<String>,
}

impl Wikilink {
    pub fn new<S: Into<String>>(target: S, label: Option<S>) -> Wikilink {
        Wikilink {
            target: target.into(),
            label: label.map(|s| s.into()),
        }
    }
}

impl Display for Wikilink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            Some(text) => f.write_fmt(format_args!("[[{}|{}]]", self.target, &text)),
            None => f.write_fmt(format_args!("[[{}]]", self.target)),
        }
    }
}

pub struct WikilinkParser {
    state: WikilinkParserState,
    current_value: Option<Wikilink>,
}

impl Default for WikilinkParser {
    fn default() -> Self {
        Self::new()
    }
}

impl WikilinkParser {
    pub fn new() -> Self {
        Self {
            state: WikilinkParserState::Start,
            current_value: None,
        }
    }

    pub fn feed(&mut self, text: &CowStr) -> Option<Wikilink> {
        match (&self.state, text) {
            (WikilinkParserState::Start, CowStr::Borrowed("[")) => {
                self.transit_state(WikilinkParserState::FirstOpen);
                None
            }
            (WikilinkParserState::FirstOpen, CowStr::Borrowed("[")) => {
                self.transit_state(WikilinkParserState::SecondOpen);
                None
            }
            (WikilinkParserState::SecondOpen, text) => {
                let wikilink = parse_wikilink_text(text);
                self.current_value = Some(wikilink);
                self.transit_state(WikilinkParserState::Text);
                None
            }
            (WikilinkParserState::Text, CowStr::Borrowed("]")) => {
                self.transit_state(WikilinkParserState::FirstClose);
                None
            }
            (WikilinkParserState::FirstClose, CowStr::Borrowed("]")) => {
                self.transit_state(WikilinkParserState::Start);
                self.current_value.clone()
            }
            _ => {
                self.transit_state(WikilinkParserState::Start);
                None
            }
        }
    }

    fn transit_state(&mut self, state: WikilinkParserState) {
        self.state = state;
    }
}

fn parse_wikilink_text(text: &str) -> Wikilink {
    let mut split = text.splitn(2, '|');
    let target = split.next().unwrap().to_string();
    let label = split.next().map(|s| s.to_string());

    Wikilink::new(target, label)
}

enum WikilinkParserState {
    Start,
    FirstOpen,
    SecondOpen,
    Text,
    FirstClose,
}

#[cfg(test)]
mod tests {
    use super::{Wikilink, WikilinkParser};
    use crate::wikilink::WikilinkParserState;
    use pulldown_cmark::CowStr;

    #[test]
    fn test_parse_wikilink() {
        let mut parser = WikilinkParser::new();
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("Page One")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("]")), None,);
        assert_eq!(
            parser.feed(&CowStr::Borrowed("]")),
            Some(Wikilink::new("Page One", None))
        );
    }

    #[test]
    fn test_parse_wikilink_label() {
        let mut parser = WikilinkParser::new();
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("Page One|Label 1")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("]")), None,);
        assert_eq!(
            parser.feed(&CowStr::Borrowed("]")),
            Some(Wikilink::new("Page One", Some("Label 1")))
        );
    }

    #[test]
    fn test_parse_wikilink_reset_state_if_an_unexpected_token_is_found() {
        let mut parser = WikilinkParser::new();
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("[")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("Page One|Label 1")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed("]")), None,);
        assert_eq!(parser.feed(&CowStr::Borrowed(" Oops")), None);
        assert!(matches!(parser.state, WikilinkParserState::Start));
    }
}
