//! HTML tokenization into a simple event stream.

use super::style::decode_entities;

/// A simple tag-based HTML event
#[derive(Debug)]
pub(crate) enum HtmlEvent {
    OpenTag {
        name: String,
        attrs: Vec<(String, String)>,
    },
    CloseTag(String),
    Text(String),
}

/// Decode common HTML entities
pub(crate) fn tokenize_html(html: &str) -> Vec<HtmlEvent> {
    let mut events = Vec::new();
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '<' {
            i += 1;
            if i >= len {
                break;
            }

            // Comment <!-- ... -->
            if i + 3 <= len
                && chars[i] == '!'
                && i + 1 < len
                && chars[i + 1] == '-'
                && i + 2 < len
                && chars[i + 2] == '-'
            {
                // skip to -->
                i += 3;
                while i + 2 < len
                    && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>')
                {
                    i += 1;
                }
                i += 3; // skip -->
                continue;
            }

            // Doctype/other <!...>
            if chars[i] == '!' {
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                i += 1;
                continue;
            }

            // Closing tag </...>
            if chars[i] == '/' {
                i += 1;
                // skip whitespace
                while i < len
                    && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n' || chars[i] == '\r')
                {
                    i += 1;
                }
                let mut name = String::new();
                while i < len && chars[i] != '>' {
                    if chars[i].is_alphanumeric()
                        || chars[i] == '-'
                        || chars[i] == ':'
                        || chars[i] == '_'
                        || chars[i] == '.'
                    {
                        name.push(chars[i]);
                    }
                    i += 1;
                }
                if i < len {
                    i += 1; // skip '>'
                }
                if !name.is_empty() {
                    events.push(HtmlEvent::CloseTag(name.to_lowercase()));
                }
                continue;
            }

            // Opening or self-closing tag
            // Skip whitespace before tag name
            while i < len
                && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n' || chars[i] == '\r')
            {
                i += 1;
            }
            let mut name = String::new();
            while i < len
                && (chars[i].is_alphanumeric()
                    || chars[i] == '-'
                    || chars[i] == ':'
                    || chars[i] == '_'
                    || chars[i] == '.')
            {
                name.push(chars[i]);
                i += 1;
            }
            let tag_name = name.to_lowercase();

            // Parse attributes
            let mut attrs: Vec<(String, String)> = Vec::new();

            while i < len && chars[i] != '>' {
                // Skip whitespace
                while i < len
                    && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n' || chars[i] == '\r')
                {
                    i += 1;
                }
                if i >= len || chars[i] == '>' {
                    break;
                }
                if chars[i] == '/' {
                    i += 1;
                    continue;
                }

                // Read attribute name
                let mut attr_name = String::new();
                while i < len
                    && chars[i] != '='
                    && chars[i] != '>'
                    && chars[i] != ' '
                    && chars[i] != '\t'
                    && chars[i] != '\n'
                    && chars[i] != '\r'
                    && chars[i] != '/'
                {
                    attr_name.push(chars[i]);
                    i += 1;
                }

                // Skip whitespace around =
                while i < len
                    && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n' || chars[i] == '\r')
                {
                    i += 1;
                }

                let mut attr_value = String::new();
                if i < len && chars[i] == '=' {
                    i += 1;
                    // Skip whitespace after =
                    while i < len
                        && (chars[i] == ' '
                            || chars[i] == '\t'
                            || chars[i] == '\n'
                            || chars[i] == '\r')
                    {
                        i += 1;
                    }
                    if i < len && (chars[i] == '"' || chars[i] == '\'') {
                        let quote = chars[i];
                        i += 1;
                        while i < len && chars[i] != quote {
                            attr_value.push(chars[i]);
                            i += 1;
                        }
                        if i < len {
                            i += 1; // skip closing quote
                        }
                    } else {
                        // unquoted value
                        while i < len
                            && chars[i] != '>'
                            && chars[i] != ' '
                            && chars[i] != '\t'
                            && chars[i] != '\n'
                            && chars[i] != '\r'
                            && chars[i] != '/'
                        {
                            attr_value.push(chars[i]);
                            i += 1;
                        }
                    }
                }

                attrs.push((attr_name.to_lowercase(), decode_entities(&attr_value)));
            }

            if i < len {
                i += 1; // skip '>'
            }

            if !tag_name.is_empty() {
                events.push(HtmlEvent::OpenTag {
                    name: tag_name,
                    attrs,
                });
            }
        } else {
            // Text content
            let mut text = String::new();
            while i < len && chars[i] != '<' {
                text.push(chars[i]);
                i += 1;
            }
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                events.push(HtmlEvent::Text(decode_entities(&text)));
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let events = tokenize_html("<h1>Hello</h1>");
        assert_eq!(events.len(), 3);
        match &events[0] {
            HtmlEvent::OpenTag { name, .. } => assert_eq!(name, "h1"),
            _ => panic!("expected OpenTag"),
        }
        match &events[1] {
            HtmlEvent::Text(t) => assert_eq!(t.trim(), "Hello"),
            _ => panic!("expected Text"),
        }
        match &events[2] {
            HtmlEvent::CloseTag(n) => assert_eq!(n, "h1"),
            _ => panic!("expected CloseTag"),
        }
    }

    #[test]
    fn test_attrs_with_single_quotes() {
        let events = tokenize_html(r#"<img src='pic.jpg' alt='hello'>"#);
        assert_eq!(events.len(), 1);
        match &events[0] {
            HtmlEvent::OpenTag { name, attrs } => {
                assert_eq!(name, "img");
                assert_eq!(
                    attrs
                        .iter()
                        .find(|(k, _)| k == "src")
                        .map(|(_, v)| v.as_str()),
                    Some("pic.jpg")
                );
                assert_eq!(
                    attrs
                        .iter()
                        .find(|(k, _)| k == "alt")
                        .map(|(_, v)| v.as_str()),
                    Some("hello")
                );
            }
            _ => panic!("expected OpenTag"),
        }
    }

    #[test]
    fn test_tokenizer_complex() {
        let events = tokenize_html(r#"<div class="main"><h1 id="title">Hello</h1></div>"#);
        assert_eq!(events.len(), 5);
        match &events[0] {
            HtmlEvent::OpenTag { name, attrs } => {
                assert_eq!(name, "div");
                assert_eq!(attrs[0].0, "class");
                assert_eq!(attrs[0].1, "main");
            }
            _ => panic!("expected OpenTag div"),
        }
    }

    #[test]
    fn test_self_closing_void_tags() {
        let events = tokenize_html(r#"<br><hr><img src="x.jpg">"#);
        assert_eq!(events.len(), 3);
        for event in &events {
            match event {
                HtmlEvent::OpenTag { name, .. } => {
                    assert!(["br", "hr", "img"].contains(&name.as_str()));
                }
                _ => panic!("expected OpenTag for void elements"),
            }
        }
    }

    #[test]
    fn test_comments_skipped() {
        let events = tokenize_html(r#"<h1>A</h1><!-- comment --><p>B</p>"#);
        // Events: OpenTag(h1), Text(A), CloseTag(h1), OpenTag(p), Text(B), CloseTag(p)
        assert_eq!(events.len(), 6);
        match &events[3] {
            HtmlEvent::OpenTag { name, .. } => assert_eq!(name, "p"),
            _ => panic!("expected p"),
        }
    }

    #[test]
    fn test_doctype_skipped() {
        let events = tokenize_html("<!DOCTYPE html><h1>Title</h1>");
        assert_eq!(events.len(), 3);
        match &events[0] {
            HtmlEvent::OpenTag { name, .. } => assert_eq!(name, "h1"),
            _ => panic!("expected h1"),
        }
    }

    #[test]
    fn test_multiple_attributes() {
        let events = tokenize_html(r#"<a href="https://x.com" class="link" id="main">text</a>"#);
        assert_eq!(events.len(), 3);
        match &events[0] {
            HtmlEvent::OpenTag { name, attrs } => {
                assert_eq!(name, "a");
                assert_eq!(attrs.len(), 3);
            }
            _ => panic!("expected OpenTag"),
        }
    }
}
