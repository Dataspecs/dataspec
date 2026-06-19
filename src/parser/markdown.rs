use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::ast::Section;

/// Parse markdown into a tree of sections keyed by heading level.
pub fn parse_sections(source: &str) -> Vec<Section> {
    let parser = Parser::new_ext(source, Options::all());
    let events: Vec<Event<'_>> = parser.collect();

    let mut root = Vec::new();
    let mut stack: Vec<Section> = Vec::new();
    let mut in_heading = false;
    let mut heading_buf = String::new();
    let mut body = String::new();
    let mut link_dest_stack: Vec<String> = Vec::new();
    let mut in_code_block = false;

    for event in &events {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                finalize_open_section(&mut root, &mut stack, &mut body);
                in_heading = true;
                heading_buf.clear();
                let section = Section {
                    level: heading_level_to_u8(*level),
                    title: String::new(),
                    body: String::new(),
                    children: Vec::new(),
                };
                attach_section(&mut root, &mut stack, section);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(current) = stack.last_mut() {
                    current.title = heading_buf.trim().to_string();
                }
                in_heading = false;
                heading_buf.clear();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                if let CodeBlockKind::Fenced(lang) = kind {
                    if !body.is_empty() && !body.ends_with('\n') {
                        body.push('\n');
                    }
                    body.push_str("```");
                    if !lang.is_empty() {
                        body.push_str(lang);
                    }
                    body.push('\n');
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                body.push_str("```\n");
            }
            Event::Text(text) => {
                if in_heading {
                    heading_buf.push_str(text);
                } else {
                    append_body(&mut body, text, in_code_block);
                }
            }
            Event::Code(text) => append_body(&mut body, text, true),
            Event::SoftBreak | Event::HardBreak => {
                if in_heading {
                    heading_buf.push(' ');
                } else {
                    body.push('\n');
                }
            }
            Event::Rule => body.push_str("\n---\n"),
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => body.push('\n'),
            Event::Start(Tag::Item) => body.push_str("- "),
            Event::End(TagEnd::Item) => body.push('\n'),
            Event::Start(Tag::Table(_)) => {}
            Event::End(TagEnd::Table) => body.push('\n'),
            Event::Start(Tag::TableHead) | Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) => body.push('|'),
            Event::End(TagEnd::TableRow) => body.push('\n'),
            Event::Start(Tag::TableCell) => body.push(' '),
            Event::End(TagEnd::TableCell) => body.push_str(" |"),
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_dest_stack.push(dest_url.to_string());
                body.push('[');
            }
            Event::End(TagEnd::Link) => {
                if let Some(dest) = link_dest_stack.pop() {
                    body.push_str("](");
                    body.push_str(&dest);
                    body.push(')');
                }
            }
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {}
            Event::Start(Tag::BlockQuote(_)) | Event::End(TagEnd::BlockQuote(_)) => {}
            Event::Start(Tag::Emphasis) | Event::End(TagEnd::Emphasis) => {}
            Event::Start(Tag::Strong) | Event::End(TagEnd::Strong) => {}
            Event::Start(Tag::Strikethrough) | Event::End(TagEnd::Strikethrough) => {}
            Event::InlineHtml(text) | Event::Html(text) => append_body(&mut body, text, false),
            Event::FootnoteReference(_)
            | Event::Start(Tag::FootnoteDefinition(_))
            | Event::End(TagEnd::FootnoteDefinition) => {}
            Event::TaskListMarker(checked) => {
                body.push_str(if *checked { "- [x] " } else { "- [ ] " });
            }
            Event::Start(Tag::MetadataBlock(_)) | Event::End(TagEnd::MetadataBlock(_)) => {}
            _ => {}
        }
    }

    finalize_open_section(&mut root, &mut stack, &mut body);
    while stack.len() > 1 {
        let finished = stack.pop().expect("stack non-empty");
        if let Some(parent) = stack.last_mut() {
            parent.children.push(finished);
        }
    }
    if let Some(last) = stack.pop() {
        root.push(last);
    }

    root
}

fn append_body(body: &mut String, text: &str, inline: bool) {
    body.push_str(text);
    if !inline {
        body.push('\n');
    }
}

fn finalize_open_section(_root: &mut Vec<Section>, stack: &mut Vec<Section>, body: &mut String) {
    if let Some(current) = stack.last_mut() {
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            if !current.body.is_empty() {
                current.body.push('\n');
            }
            current.body.push_str(trimmed);
        }
    }
    body.clear();
}

fn attach_section(root: &mut Vec<Section>, stack: &mut Vec<Section>, section: Section) {
    while let Some(top) = stack.last() {
        if top.level >= section.level {
            let finished = stack.pop().expect("stack non-empty");
            if let Some(parent) = stack.last_mut() {
                parent.children.push(finished);
            } else {
                root.push(finished);
            }
        } else {
            break;
        }
    }
    stack.push(section);
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_tree() {
        let md = "# root\nintro\n\n## Type\nmodel\n\n## Columns\n### col1\ndesc\n#### Type\nString\n";
        let sections = parse_sections(md);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].title, "root");
        assert!(sections[0].body.contains("intro"));
        let ty = sections[0].child("Type").unwrap();
        assert_eq!(ty.body_trimmed(), "model");
        let columns = sections[0].child("Columns").unwrap();
        let col1 = columns.child("col1").unwrap();
        assert_eq!(col1.body_trimmed(), "desc");
        assert_eq!(col1.child("Type").unwrap().body_trimmed(), "String");
    }
}
