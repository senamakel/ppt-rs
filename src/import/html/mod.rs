//! HTML to PowerPoint conversion
//!
//! Parses HTML content and converts it into PowerPoint slide structures.
//! No external dependencies required - uses a lightweight state-machine parser.
//!
//! # This is the Basic HTML Parser
//!
//! This parser is designed for simple, fast HTML-to-PowerPoint conversion
//! with minimal dependencies. For advanced web scraping and content extraction,
//! see the `web2ppt::parser` module (requires scraper crate).
//!
//! ## When to use this parser:
//! - Converting simple HTML strings
//! - Processing well-structured HTML files
//! - When you want zero external dependencies
//! - For embedded applications
//!
//! ## When to use web2ppt::parser instead:
//! - Processing live web pages with navigation/ads
//! - When you need intelligent content extraction
//! - For automatic content cleaning and detection
//! - When you need to handle complex web page structures
//!
//! # Supported HTML elements
//!
//! - `<h1>` → New slide title
//! - `<h2>` through `<h6>` → Bold section headers on current slide
//! - `<p>` → Bullet points / paragraphs
//! - `<ul>/<ol>` with `<li>` → List items
//! - `<table>` with `<tr>/<th>/<td>` → Table objects (styled header row)
//! - `<pre>/<code>` → Code blocks
//! - `<img>` → Image placeholders (with alt text)
//! - `<blockquote>` → Speaker notes
//! - `<strong>/<b>` → Bold text (via markdown-style `**`)
//! - `<em>/<i>` → Italic text (via markdown-style `*`)
//! - `<a href="...">` → Hyperlink text
//! - `<hr>` → Slide break
//! - `<br>` → Line break within text
//! - `<title>` → Presentation title (falls back to first `<h1>`)

mod parser;
mod style;
mod tokenizer;

use crate::generator::SlideContent;

/// Options for HTML parsing
#[derive(Clone, Debug)]
pub struct HtmlParseOptions {
    /// Maximum slides to generate
    pub max_slides: usize,
    /// Maximum bullet points per slide
    pub max_bullets: usize,
    /// Include code blocks
    pub include_code: bool,
    /// Include tables
    pub include_tables: bool,
    /// Include image placeholders
    pub include_images: bool,
}

impl Default for HtmlParseOptions {
    fn default() -> Self {
        Self {
            max_slides: 50,
            max_bullets: 10,
            include_code: true,
            include_tables: true,
            include_images: true,
        }
    }
}

impl HtmlParseOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_slides(mut self, n: usize) -> Self {
        self.max_slides = n;
        self
    }

    pub fn max_bullets(mut self, n: usize) -> Self {
        self.max_bullets = n;
        self
    }

    pub fn include_code(mut self, include: bool) -> Self {
        self.include_code = include;
        self
    }

    pub fn include_tables(mut self, include: bool) -> Self {
        self.include_tables = include;
        self
    }

    pub fn include_images(mut self, include: bool) -> Self {
        self.include_images = include;
        self
    }
}

/// Parse HTML content into slides with default options
pub fn parse_html(html: &str) -> Result<Vec<SlideContent>, String> {
    Html2Ppt::with_options(HtmlParseOptions::default()).parse(html)
}

/// Parse HTML content into slides with custom options
pub fn parse_html_with_options(
    html: &str,
    options: HtmlParseOptions,
) -> Result<Vec<SlideContent>, String> {
    Html2Ppt::with_options(options).parse(html)
}


// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// HTML to PowerPoint converter
pub struct Html2Ppt {
    options: HtmlParseOptions,
}

impl Html2Ppt {
    pub fn new() -> Self {
        Self::with_options(HtmlParseOptions::default())
    }

    pub fn with_options(options: HtmlParseOptions) -> Self {
        Self { options }
    }

    /// Parse an HTML string into slide content
    pub fn parse(&self, html: &str) -> Result<Vec<SlideContent>, String> {
        let events = tokenizer::tokenize_html(html);
        parser::HtmlSlideParser::new(self.options.clone()).parse(&events)
    }

    /// Parse HTML from a file path
    pub fn parse_file(&self, path: &str) -> Result<Vec<SlideContent>, String> {
        let html =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read HTML file: {e}"))?;
        self.parse(&html)
    }
}

impl Default for Html2Ppt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_headings() {
        let html = "<h1>First Slide</h1><p>Some content</p><h1>Second Slide</h1>";
        let slides = parse_html(html).unwrap();
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].title, "First Slide");
        assert_eq!(slides[1].title, "Second Slide");
        assert_eq!(slides[1].content.len(), 0);
    }

    #[test]
    fn test_table() {
        let html = r#"
            <html><body>
                <h1>Data</h1>
                <table>
                    <tr><th>Name</th><th>Value</th></tr>
                    <tr><td>A</td><td>1</td></tr>
                    <tr><td>B</td><td>2</td></tr>
                </table>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].table.is_some());
    }

    #[test]
    fn test_code_block() {
        let html = r#"
            <html><body>
                <h1>Code Example</h1>
                <pre><code>fn main() { println!("hello"); }</code></pre>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert!(!slides[0].code_blocks.is_empty());
        assert!(slides[0].code_blocks[0].code.contains("fn main()"));
    }

    #[test]
    fn test_blockquote_notes() {
        let html = r#"
            <html><body>
                <h1>Slide</h1>
                <p>Content</p>
                <blockquote>Speaker note here</blockquote>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].notes, Some("Speaker note here".to_string()));
    }

    #[test]
    fn test_hr_slide_break() {
        let html = "<h1>Slide 1</h1><p>Content</p><hr><h1>Slide 2</h1><p>More content</p>";
        let slides = parse_html(html).unwrap();
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_entity_decoding() {
        let html = r#"
            <html><body>
                <h1>Test</h1>
                <p>AT&amp;T &lt;test&gt; &quot;quote&quot;</p>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].content[0].contains("AT&T"));
        assert!(slides[0].content[0].contains("<test>"));
    }

    #[test]
    fn test_img_placeholder() {
        let html = r#"
            <html><body>
                <h1>Images</h1>
                <img src="photo.jpg" alt="A photo">
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert!(
            slides[0]
                .content
                .iter()
                .any(|c| c.contains("[Image: A photo]"))
        );
    }

    #[test]
    fn test_skip_script_style() {
        let html = r#"
            <html><body>
                <h1>Real Content</h1>
                <p>Visible text</p>
                <script>var x = "should not appear";</script>
                <style>.hidden { color: red; }</style>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].content.len(), 1);
        assert!(slides[0].content[0].contains("Visible"));
    }

    #[test]
    fn test_no_h1_fallback() {
        let html = r#"<html><body><p>Just a paragraph.</p></body></html>"#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_empty_input() {
        let result = parse_html("<html><body></body></html>");
        assert!(result.is_err());
    }

    #[test]
    fn test_br_tag() {
        let html = r#"<html><body><h1>Title</h1><p>Line 1<br>Line 2</p></body></html>"#;
        let slides = parse_html(html).unwrap();
        assert!(!slides[0].content.is_empty());
    }

    #[test]
    fn test_bold_italic() {
        let html = r#"
            <html><body>
                <h1>Formatting</h1>
                <p><strong>Bold</strong> and <em>italic</em> text</p>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        let c = &slides[0].content[0];
        assert!(c.contains("**Bold**"));
    }

    #[test]
    fn test_complex_nested() {
        let html = r#"
            <html><body>
                <h1>Welcome</h1>
                <p>Introduction paragraph.</p>
                <h2>Section A</h2>
                <ul>
                    <li>First item</li>
                    <li>Second item</li>
                </ul>
                <h1>Details</h1>
                <table><tr><th>Col1</th><th>Col2</th></tr>
                       <tr><td>A</td><td>B</td></tr></table>
                <pre><code>let x = 1;</code></pre>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[0].title, "Welcome");
        assert!(!slides[1].code_blocks.is_empty());
        assert!(slides[1].table.is_some());
    }

    #[test]
    fn test_html2ppt_options() {
        let options = HtmlParseOptions::new()
            .max_slides(3)
            .max_bullets(5)
            .include_images(false);
        assert_eq!(options.max_slides, 3);
        assert_eq!(options.max_bullets, 5);
        assert!(!options.include_images);
    }

    #[test]
    fn test_html2ppt_struct() {
        let converter = Html2Ppt::new();
        let html = "<h1>Test</h1><p>Content</p>";
        let slides = converter.parse(html).unwrap();
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_nested_elements() {
        let html = r#"
            <div><div><div><div><div>
                <h1>Deep Nesting</h1>
                <p>Still works</p>
            </div></div></div></div></div>
        "#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].title, "Deep Nesting");
    }

    #[test]
    fn test_link_with_href() {
        let html = r#"
            <html><body>
                <h1>Links</h1>
                <p>Visit <a href="https://example.com">Example</a> website</p>
            </body></html>
        "#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].content[0].contains("Example"));
    }

    #[test]
    fn test_paragraph_inline_color() {
        let html = r#"<h1>Test</h1><p style="color:red">Red text</p>"#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].bullets.len(), 1);
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.color, Some("FF0000".to_string()));
    }

    #[test]
    fn test_paragraph_inline_font_size() {
        let html = r#"<h1>Test</h1><p style="font-size:20px">Bigger text</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.font_size, Some(15));
    }

    #[test]
    fn test_paragraph_inline_bold() {
        let html = r#"<h1>Test</h1><p style="font-weight:bold">Bold paragraph</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert!(fmt.bold);
    }

    #[test]
    fn test_paragraph_inline_italic() {
        let html = r#"<h1>Test</h1><p style="font-style:italic">Italic paragraph</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert!(fmt.italic);
    }

    #[test]
    fn test_paragraph_inline_underline() {
        let html = r#"<h1>Test</h1><p style="text-decoration:underline">Underlined</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert!(fmt.underline);
    }

    #[test]
    fn test_paragraph_inline_multiple_styles() {
        let html =
            r#"<h1>Test</h1><p style="color:blue; font-size:18pt; font-weight:bold">Styled</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.color, Some("0000FF".to_string()));
        assert_eq!(fmt.font_size, Some(18));
        assert!(fmt.bold);
    }

    #[test]
    fn test_paragraph_no_style_no_format() {
        let html = "<h1>Test</h1><p>Plain text</p>";
        let slides = parse_html(html).unwrap();
        assert!(slides[0].bullets[0].format.is_none());
    }

    #[test]
    fn test_h1_inline_color() {
        let html = r#"<h1 style="color:green">Green Title</h1>"#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].title_color, Some("008000".to_string()));
    }

    #[test]
    fn test_h1_inline_font_size() {
        let html = r#"<h1 style="font-size:36pt">Big Title</h1>"#;
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].title_size, Some(36));
    }

    #[test]
    fn test_h1_inline_bold_true() {
        let html = r#"<h1 style="font-weight:bold">Bold Title</h1>"#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].title_bold); // True because css bold=True; default is also true
    }

    #[test]
    fn test_h1_inline_italic() {
        let html = r#"<h1 style="font-style:italic">Italic Title</h1>"#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].title_italic);
    }

    #[test]
    fn test_h1_underline_from_style() {
        let html = r#"<h1 style="text-decoration:underline">Underlined Title</h1>"#;
        let slides = parse_html(html).unwrap();
        assert!(slides[0].title_underline);
    }

    #[test]
    fn test_list_item_with_inline_style() {
        let html =
            r#"<h1>List</h1><ul><li style="color:red">Red item</li><li>Normal item</li></ul>"#;
        let slides = parse_html(html).unwrap();
        let fmt0 = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("First item should have format");
        assert_eq!(fmt0.color, Some("FF0000".to_string()));
        assert!(slides[0].bullets[1].format.is_none()); // second item has no style
    }

    #[test]
    fn test_nested_style_inheritance() {
        let html =
            r#"<div style="color:red"><p>Red text</p><p style="color:blue">Blue text</p></div>"#;
        let slides = parse_html(html).unwrap();
        // Both paragraphs ended up as bullets on the same slide (if first h1 existed or auto-title)
        assert_eq!(slides[0].bullets.len(), 2);
        let fmt0 = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("First should have format");
        assert_eq!(fmt0.color, Some("FF0000".to_string())); // inherits red from div
        let fmt1 = slides[0].bullets[1]
            .format
            .as_ref()
            .expect("Second should have format");
        assert_eq!(fmt1.color, Some("0000FF".to_string())); // overrides to blue
    }

    #[test]
    fn test_style_on_container_div() {
        let html =
            r#"<h1>Styled Container</h1><div style="color:purple"><p>Purple paragraph</p></div>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.color, Some("800080".to_string()));
    }

    #[test]
    fn test_void_tag_br_does_not_affect_style() {
        let html = r#"<h1>Test</h1><p style="color:red">First<br style="color:blue">Second</p>"#;
        let slides = parse_html(html).unwrap();
        // The <br> should not push/pop style, so the paragraph should have color:red
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.color, Some("FF0000".to_string()));
    }

    #[test]
    fn test_style_content_size_default() {
        let html = "<h1>Test</h1><p>Default size</p>";
        let slides = parse_html(html).unwrap();
        assert_eq!(slides[0].content_size, Some(28));
    }

    #[test]
    fn test_background_color_as_highlight() {
        let html = r#"<h1>Test</h1><p style="background-color:yellow">Highlighted</p>"#;
        let slides = parse_html(html).unwrap();
        let fmt = slides[0].bullets[0]
            .format
            .as_ref()
            .expect("Should have format");
        assert_eq!(fmt.highlight, Some("FFFF00".to_string()));
    }
}
