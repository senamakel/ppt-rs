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
