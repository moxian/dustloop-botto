use parse_wiki_text::Node;

pub fn transform_text(
    existing_text: &str,
    visitor: &mut impl WikiVisitor,
    skip_errors: bool,
) -> anyhow::Result<String> {
    let parsed = parse_wiki_text::Configuration::new(&parse_wiki_text::ConfigurationSource {
        link_trail: "/^([a-z]+)(.*)$/sD",

        category_namespaces: &[],
        extension_tags: &[
            "big",
            "pre",
            "nowiki",
            "gallery",
            "indicator",
            "section",
            "categorytree",
            "imagemap",
            "ref",
            "references",
            "templatedata",
            "embedvideo",
            "archiveorg",
            "soundcloud",
            "spotifyalbum",
            "spotifyartist",
            "spotifytrack",
            "twitch",
            "twitchclip",
            "twitchvod",
            "vimeo",
            "youtubeoembed",
            "youtube",
            "youtubeplaylist",
            "youtubevideolist",
            "tabber",
            "tabbertransclude",
            "seo",
        ],
        file_namespaces: &[],
        magic_words: &[],
        protocols: &[],
        redirect_magic_words: &[],
    })
    .parse(existing_text);

    let warnings = parsed
        .warnings
        .into_iter()
        .filter(|w| {
            ![
                parse_wiki_text::WarningMessage::StrayTextInTable,
                parse_wiki_text::WarningMessage::RepeatedEmptyLine,
                parse_wiki_text::WarningMessage::InvalidLinkSyntax, // huh??
                parse_wiki_text::WarningMessage::UnrecognizedTagName, // i'm tired
            ]
            .contains(&w.message)
        })
        .filter(|w| {
            let text = String::from_utf8_lossy(
                &existing_text.as_bytes()[w.start..w.end.min(existing_text.len())],
            )
            .to_string();
            if w.message == parse_wiki_text::WarningMessage::UnrecognizedTagName {
                if ["", "SlashGordon", "),"].contains(&text.as_str()) {
                    return false;
                }
                if text.as_str().starts_with("=") {
                    // <=
                    return false;
                }
                if text.as_str().chars().next().unwrap().is_digit(10) {
                    // <123frames or something
                    return false;
                }
            }
            return true;
        })
        .collect::<Vec<_>>();

    for w in &warnings {
        let msg = &existing_text[w.start..w.end.min(existing_text.len())];
        let snippet = &existing_text[(w.start.max(10) - 10)..(w.end + 10).min(existing_text.len())];
        let msg = &msg[..msg.len().min(500)];
        println!("{}: {}", w.message, msg);
        println!(".. around: {}", snippet);
    }
    if !skip_errors {
        assert!(warnings.is_empty());
    }

    // let mut visitor = ColorVisitor::default();
    visitor.set_base_text(existing_text);
    visit_nodes(visitor, &parsed.nodes, existing_text);

    let mut replacements = visitor.get_replacements()?.to_vec();
    replacements.sort_by_key(|(_, r)| r.start);
    {
        let mut last = 0;
        for r in &replacements {
            assert!(last < r.1.start);
            last = r.1.end;
        }
    }
    let mut out = String::new();
    let mut last = 0;
    for (rep, rang) in &replacements {
        out += &existing_text[last..rang.start];
        out += rep;
        last = rang.end;
    }
    out += &existing_text[last..];

    Ok(out)
}

#[allow(unused_variables)]
pub trait WikiVisitor {
    fn set_base_text(&mut self, base_text: &str);
    fn get_replacements(&self) -> anyhow::Result<&[(String, std::ops::Range<usize>)]>; // split into dedicated trait if needed

    fn visit_template(&mut self, node: &Node) {}
    fn visit_table_start(&mut self, node: &Node) {}
    fn visit_table_row(&mut self, row: &parse_wiki_text::TableRow) {}
    fn visit_table_end(&mut self, node: &Node) {}

    fn visit_start_tag(&mut self, node: &Node) {}
    fn visit_heading(&mut self, node: &Node) {}
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
        Node::Heading { nodes, .. } => {
            visitor.visit_heading(node);
            visit_nodes(visitor, nodes, existing_text);
        }
        Node::Tag { nodes, .. }
        | Node::Link { text: nodes, .. }
        | Node::Preformatted { nodes, .. } => {
            for n in nodes {
                visit_node(visitor, &n, existing_text)
            }
        }
        Node::StartTag { .. } => {
            visitor.visit_start_tag(node);
        }
        Node::Text { .. }
        | Node::EndTag { .. }
        | Node::Comment { .. }
        | Node::ParagraphBreak { .. }
        | Node::HorizontalDivider { .. }
        | Node::Italic { .. }
        | Node::Bold { .. }
        | Node::CharacterEntity { .. }
        | Node::BoldItalic { .. } => {}
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
        Node::UnorderedList { items, .. } | Node::OrderedList { items, .. } => {
            for item in items {
                for n in &item.nodes {
                    visit_node(visitor, &n, existing_text)
                }
            }
        }
        Node::DefinitionList { items, .. } => {
            for it in items {
                visit_nodes(visitor, &it.nodes, existing_text)
            }
        }
        Node::Parameter { default, name, .. } => {
            visit_nodes(visitor, name, existing_text);
            if let Some(d) = default {
                visit_nodes(visitor, d, existing_text);
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
