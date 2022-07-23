use parse_wiki_text::Node;



#[allow(unused_variables)]
pub trait WikiVisitor {
    fn set_base_text(&mut self, base_text: &str);
    fn visit_template(&mut self, node: &Node) {}

    fn visit_table_start(&mut self, node: &Node) {}
    fn visit_table_row(&mut self, row: &parse_wiki_text::TableRow) {}
    fn visit_table_end(&mut self, node: &Node) {}
}


fn visit_node(visitor: &mut impl WikiVisitor, node: &parse_wiki_text::Node, existing_text: &str) {
    match &node {
        Node::Template {
            name, parameters, ..
        } => {
            visitor.visit_template(node);
            for n in name {
                visit_node(visitor, n, existing_text);
            }
            for param in parameters {
                if let Some(ns) = &param.name {
                    visit_nodes(visitor, ns, existing_text);
                }
                visit_nodes(visitor, &param.value, existing_text);
            }
        }
        Node::Heading { nodes, .. } | Node::Tag { nodes, .. } | Node::Link { text: nodes, .. } => {
            for n in nodes {
                visit_node(visitor, &n, existing_text)
            }
        }
        Node::Text { .. }
        | Node::StartTag { .. }
        | Node::EndTag { .. }
        | Node::Comment { .. }
        | Node::ParagraphBreak { .. }
        | Node::HorizontalDivider { .. }
        | Node::Italic { .. }
        | Node::Bold { .. } => {}
        Node::Table {
            attributes,
            captions,
            rows,
            ..
        } => {
            visitor.visit_table_start(node);
            for n in attributes {
                visit_node(visitor, &n, existing_text)
            }
            for cap in captions {
                if let Some(atts) = &cap.attributes {
                    for n in atts {
                        visit_node(visitor, &n, existing_text)
                    }
                }
                for n in &cap.content {
                    visit_node(visitor, &n, existing_text)
                }
            }
            for row in rows {
                visitor.visit_table_row(row);
                for n in &row.attributes {
                    visit_node(visitor, &n, existing_text)
                }
                for cell in &row.cells {
                    if let Some(atts) = &cell.attributes {
                        for n in atts {
                            visit_node(visitor, &n, existing_text)
                        }
                    }
                    for n in &cell.content {
                        visit_node(visitor, &n, existing_text)
                    }
                }
            }
            visitor.visit_table_end(node);
        }
        Node::UnorderedList { items, .. } => {
            for item in items {
                for n in &item.nodes {
                    visit_node(visitor, &n, existing_text)
                }
            }
        }
        Node::DefinitionList { items, .. } => {
            for it in items {
                for n in &it.nodes {
                    visit_node(visitor, &n, existing_text)
                }
            }
        }

        _ => panic!("unhandled type: {:?}", node),
    }
}

pub fn visit_nodes(
    visitor: &mut impl WikiVisitor,
    nodes: &[parse_wiki_text::Node],
    existing_text: &str,
) {
    for node in nodes {
        visit_node(visitor, node, existing_text)
    }
}