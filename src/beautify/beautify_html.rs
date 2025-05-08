use regex::Regex;

use crate::{
    parser::{html_document::Node, html_parse::parse_html_document},
    services::html_formatter::HTMLFormatConfiguration,
    HTMLDataManager,
};

pub fn html_beautify(
    content: &str,
    options: &HTMLFormatConfiguration,
    case_sensitive: bool,
) -> String {
    let html_document =
        parse_html_document(content, "html", &HTMLDataManager::default(), case_sensitive);
    let mut formated = String::new();
    for root in &html_document.roots {
        formated.push_str(&beautify_node(content, root, options, 0));
    }
    if !formated.ends_with('\n') && options.end_with_newline {
        formated += "\n";
    }

    formated
}

fn beautify_node(
    content: &str,
    node: &Node,
    options: &HTMLFormatConfiguration,
    level: usize,
) -> String {
    let tag = node.tag.as_ref().unwrap();
    let mut attrs_format = String::new();
    let attrs_is_wrap = node_attrs_is_wrap(&node, level, options);
    for name in node.attribute_names_by_order() {
        let mut value = None;
        if let Some(v) = node.attributes.get(name) {
            value = v.value.clone();
        }
        if let Some(value) = value {
            if attrs_is_wrap {
                attrs_format.push_str(&format!(
                    "\n{}{}={}",
                    get_attr_indent(options, level),
                    name,
                    value
                ));
            } else {
                attrs_format.push_str(&format!(" {}={}", name, value));
            }
        } else {
            if attrs_is_wrap {
                attrs_format.push_str(&format!("\n{}{}", get_attr_indent(options, level), name));
            } else {
                attrs_format.push_str(&format!(" {}", name));
            }
        }
    }
    let indent = get_indent(options, level);
    if is_self_closing(&node) {
        if attrs_is_wrap {
            format!("{}<{}{}\n{}/>", indent, tag, attrs_format, indent)
        } else {
            format!("{}<{}{} />", indent, tag, attrs_format)
        }
    } else {
        let mut children = String::new();
        let start_tag_end = node.start_tag_end.unwrap();
        let end_tag_start = node.end_tag_start.unwrap();
        let mut prev_child_end = start_tag_end;
        for (i, child) in node.children.iter().enumerate() {
            // before text of each child
            let text = &content[prev_child_end..child.start];
            children.push_str(&beautify_text(text, level + 1, options));
            prev_child_end = child.end;
            // child
            children.push_str(&format!(
                "\n{}",
                beautify_node(content, &child, options, level + 1)
            ));
            // after text of last child
            if i == node.children.len() - 1 {
                let text = &content[prev_child_end..node.end_tag_start.unwrap()];
                children.push_str(&beautify_text(text, level + 1, options));
            }
        }
        let is_wrap = node_is_wrap(&node, level, content, options);
        if node.children.len() == 0 && start_tag_end != end_tag_start {
            let text = &content[start_tag_end..end_tag_start];
            let text = beautify_text(text, level + 1, options);
            if is_wrap && text.trim().len() > 0 {
                children.push_str(&format!(
                    "\n{}{}",
                    get_indent(options, level + 1),
                    text.trim_start()
                ));
            } else {
                children.push_str(&text);
            }
        }
        if attrs_is_wrap {
            format!(
                "{}<{}{}\n{}>{}\n{}</{}>",
                indent, tag, attrs_format, indent, children, indent, tag
            )
        } else if is_wrap {
            format!(
                "{}<{}{}>{}\n{}</{}>",
                indent, tag, attrs_format, children, indent, tag
            )
        } else {
            format!("{}<{}{}>{}</{}>", indent, tag, attrs_format, children, tag)
        }
    }
}

fn beautify_text(text: &str, level: usize, options: &HTMLFormatConfiguration) -> String {
    let whitespace_reg = Regex::new("\\s+").unwrap();

    if text.contains('\n') {
        let mut result = String::new();
        let lines = text.lines();
        let count = lines.clone().count();
        let mut preserve_count = 0;
        for (i, line) in lines.enumerate() {
            let line = whitespace_reg.replace_all(line.trim(), " ");
            if line.len() > 0 {
                result.push_str(&format!("\n{}{}", get_indent(options, level), line));
                preserve_count = 0;
            } else if i != 0
                && (i != count - 1 || text.ends_with("\n"))
                && options.preserve_new_lines
                && (options.max_preserve_new_lines.is_none()
                    || options
                        .max_preserve_new_lines
                        .is_some_and(|v| v > preserve_count))
            {
                result.push_str("\n");
                if options.indent_empty_lines {
                    result.push_str(&get_indent(options, level));
                }
                preserve_count += 1;
            } else if i != 0 {
                preserve_count += 1;
            }
        }
        result
    } else {
        whitespace_reg.replace_all(text.trim(), " ").to_string()
    }
}

fn get_indent(options: &HTMLFormatConfiguration, level: usize) -> String {
    if options.insert_spaces {
        " ".repeat(options.tab_size as usize * level)
    } else {
        "\t".to_string().repeat(level)
    }
}

fn get_attr_indent(options: &HTMLFormatConfiguration, level: usize) -> String {
    let mut indent = get_indent(options, level);
    if let Some(indent_size) = options.wrap_attributes_indent_size {
        if options.insert_spaces {
            indent += &" ".repeat(indent_size as usize);
        } else if indent_size > 0 {
            indent += "\n";
        }
    } else {
        if options.insert_spaces {
            indent += &" ".repeat(options.tab_size as usize);
        } else {
            indent += "\n";
        }
    }
    indent
}

fn is_self_closing(node: &Node) -> bool {
    node.end_tag_start.is_none()
}

fn node_is_wrap(
    node: &Node,
    level: usize,
    content: &str,
    options: &HTMLFormatConfiguration,
) -> bool {
    if let Some(start_tag_end) = node.start_tag_end {
        if let Some(end_tag_start) = node.end_tag_start {
            if content[start_tag_end..end_tag_start].contains('\n') {
                return true;
            }
        }
    }

    if options.wrap_line_length.is_none() {
        return false;
    }

    let tag = if let Some(tag) = &node.tag {
        tag
    } else {
        return false;
    };

    let left_tag_len = get_left_tag_len(node, level, options).unwrap();
    let total = if node.is_self_closing() {
        left_tag_len
    } else {
        let content_len = node.end_tag_start.unwrap() - node.start_tag_end.unwrap();
        let end_left_bracket_len = 2;
        left_tag_len + content_len + end_left_bracket_len + tag.len() + 2
    };

    total > options.wrap_line_length.unwrap()
}

fn node_attrs_is_wrap(node: &Node, level: usize, options: &HTMLFormatConfiguration) -> bool {
    if options.wrap_line_length.is_none() {
        return false;
    }

    if let Some(total) = get_left_tag_len(node, level, options) {
        total > options.wrap_line_length.unwrap()
    } else {
        false
    }
}

fn get_left_tag_len(node: &Node, level: usize, options: &HTMLFormatConfiguration) -> Option<usize> {
    let tag = if let Some(tag) = &node.tag {
        tag
    } else {
        return None;
    };

    let indent = get_indent(options, level).len();
    let left_bracket_len = 1;
    let right_bracket_len = 1;
    let right_self_closing = 3; // include one space
    let mut attrs_len = 0;
    for name in node.attribute_names() {
        if let Some(value) = &node.attributes.get(name).unwrap().value {
            attrs_len += 1 + name.len() + 1 + value.len(); // { name="value"}
        } else {
            attrs_len += 1 + name.len();
        }
    }

    if node.is_self_closing() {
        Some(indent + left_bracket_len + tag.len() + attrs_len + right_self_closing)
    } else {
        Some(indent + left_bracket_len + tag.len() + attrs_len + right_bracket_len)
    }
}
