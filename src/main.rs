use parse_wiki_text::Node;

const WIKI_URL: &str = "https://www.dustloop.com/wiki/api.php";

#[derive(serde::Deserialize)]
struct Cred {
    name: String,
    password: String,
}

async fn get_existing_page_text(api: &mediawiki::api::Api, page: &str) -> Option<String> {
    let params = api.params_into(&[
        ("action", "parse"),
        ("page", page),
        ("prop", "wikitext"),
        ("formatversion", "2"), //cargo cult
    ]);
    let res = api.post_query_api_json(&params).await.unwrap();
    let err = &res.as_object().unwrap().get("error");
    if let Some(err) = err {
        let code = err.as_object().unwrap()["code"].as_str().unwrap();
        if code == "missingtitle" {
            return None;
        }
        panic!("{:?}", err);
    }
    let text = res.as_object().unwrap()["parse"].as_object().unwrap()["wikitext"]
        .as_str()
        .unwrap()
        .to_string();
    Some(text)
}

trait NodeExt {
    fn as_str(&self) -> &str;
    fn range(&self) -> std::ops::Range<usize>;
    fn start(&self) -> usize;
    fn end(&self) -> usize;
}
impl NodeExt for parse_wiki_text::Node<'_> {
    fn as_str(&self) -> &str {
        match self {
            parse_wiki_text::Node::Text { value, .. } => value,
            _ => panic!("{:?} is not a text node", self),
        }
    }
    fn range(&self) -> std::ops::Range<usize> {
        match self {
            Node::Text { start, end, .. }
            | Node::Link { start, end, .. }
            | Node::Template { start, end, .. }
            | Node::Comment { start, end, .. } => *start..*end,

            _ => unimplemented!("Node: {:?}", self),
        }
    }
    fn start(&self) -> usize {
        self.range().start
    }
    fn end(&self) -> usize {
        self.range().end
    }
}

trait ParameterExt {
    fn as_str<'a>(&self, source: &'a str) -> &'a str;
    fn name_str<'a>(&self, source: &'a str) -> &'a str;
    fn val_str<'a>(&self, source: &'a str) -> &'a str;
}
impl ParameterExt for parse_wiki_text::Parameter<'_> {
    fn as_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    fn name_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.name.as_ref().unwrap().first().unwrap().start()
            ..self.name.as_ref().unwrap().last().unwrap().end()]
    }
    fn val_str<'a>(&'_ self, source: &'a str) -> &'a str {
        if self.value.len() == 0 {
            ""
        } else {
            let first = self.value.first().unwrap();
            let last = self.value.last().unwrap();
            &source[first.start()..last.end()]
        }
    }
}

trait TableCellExt {
    fn text_content<'a>(&self, source: &'a str) -> &'a str;
}
impl TableCellExt for parse_wiki_text::TableCell<'_> {
    fn text_content<'a>(&self, source: &'a str) -> &'a str {
        if self.content.is_empty() {
            return "";
        }
        &source[self.content.first().unwrap().start()..self.content.last().unwrap().end()]
    }
}

#[allow(unused_variables)]
trait WikiVisitor {
    fn set_base_text(&mut self, base_text: &str);
    fn visit_template(&mut self, node: &Node) {}

    fn visit_table_start(&mut self, node: &Node) {}
    fn visit_table_row(&mut self, row: &parse_wiki_text::TableRow) {}
    fn visit_table_end(&mut self, node: &Node) {}
}

#[derive(Default)]
struct ColorVisitor {
    base_text: String,
    replacements: Vec<(&'static str, std::ops::Range<usize>)>,
}
impl WikiVisitor for ColorVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
    }
    fn visit_template(&mut self, node: &parse_wiki_text::Node) {
        match node {
            Node::Template {
                name, parameters, ..
            } => {
                let name_text =
                    &self.base_text[name.first().unwrap().start()..name.last().unwrap().end()];
                match name_text {
                    "clr" | "color" => {
                        let color = parameters[0].as_str(&self.base_text);
                        let letter_color = match color {
                            "1" | "P" => "P",
                            "2" | "K" => "K",
                            "3" | "S" => "S",
                            "4" | "H" => "H",
                            "5" | "D" => "D",
                            _ => panic!("unknown color {:?}", color),
                        };
                        // println!(
                        //     "hi we are at around {:?}",
                        //     &existing_text[(node.start().max(10) - 10)
                        //         ..((node.end() + 10).min(existing_text.len()))]
                        // );
                        self.replacements
                            .push((letter_color, parameters[0].start..parameters[0].end));
                    }
                    _ => {}
                }
            }
            _ => unreachable!(),
        };
    }
}

#[derive(Default)]
struct ComboTableVisitor {
    base_text: String,
    in_table: bool,
    column_order: Option<Vec<String>>,
    replacements: Vec<(String, std::ops::Range<usize>)>,
}

impl WikiVisitor for ComboTableVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
    }
    fn visit_table_start(&mut self, _: &Node) {
        assert!(!self.in_table);
        self.in_table = true;
        self.column_order = None;
    }
    fn visit_table_end(&mut self, _: &Node) {
        assert!(self.in_table);
        self.in_table = false;
    }
    fn visit_table_row(&mut self, row: &parse_wiki_text::TableRow) {
        if row.cells.len() == 0 {
            //either the last row OR templatized already
            return;
        }
        if row.cells.len() <= 2 {
            panic!("idk")
        }
        let mut out = String::new();
        if self.column_order.is_none() {
            let mut order = vec![];
            // must be the heading
            assert!(row
                .cells
                .iter()
                .all(|c| c.type_ == parse_wiki_text::TableCellType::Heading));
            for cell in &row.cells {
                let caption = cell.text_content(&self.base_text);
                order.push(match caption.to_lowercase().as_str() {
                    "combo" => "combo",
                    "position" => "position",
                    "damage" => "damage",
                    "tension gain" => "tensionGain",
                    "works on:" => "worksOn",
                    "difficulty" => "difficulty",
                    "video" => "video",
                    "notes" => "notes",
                    _ => panic!("unknown caption {:?}", caption),
                });
            }
            self.column_order = Some(order.into_iter().map(|x| x.to_string()).collect());
            out += "|-\n{{GGST-ComboTableHeader}}"
        } else {
            let desired_order = [
                "combo",
                "position",
                "damage",
                "tensionGain",
                "worksOn",
                "difficulty",
                "notes",
                "video",
            ];
            let mut kvs = std::collections::BTreeMap::new();
            assert_eq!(row.cells.len(), self.column_order.as_ref().unwrap().len());
            for (cell, column) in row.cells.iter().zip(self.column_order.as_ref().unwrap()) {
                kvs.insert(column.as_str(), cell.text_content(&self.base_text));
            }

            out += "|-\n{{GGST-ComboTableRow\n";
            for col in desired_order {
                let val = kvs.remove(col);
                if let Some(val) = val {
                    if col == "video" && val == "-" {
                        continue;
                    }
                    out += &format!("|{} = {}\n", col, val)
                }
            }
            out += "}}";
            assert!(kvs.is_empty(), "Some keys left over! {:?}", kvs);
        }

        self.replacements.push((out, row.start..row.end));
    }
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

fn visit_nodes(
    visitor: &mut impl WikiVisitor,
    nodes: &[parse_wiki_text::Node],
    existing_text: &str,
) {
    for node in nodes {
        visit_node(visitor, node, existing_text)
    }
}

fn colorize_page(existing_text: &str) -> String {
    let parsed = parse_wiki_text::Configuration::new(&parse_wiki_text::ConfigurationSource {
        link_trail: "/^([a-z]+)(.*)$/sD",

        category_namespaces: &[],
        extension_tags: &["big", "tabber", "gallery", "nowiki"],
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
            ]
            .contains(&w.message)
        })
        .collect::<Vec<_>>();
    for w in &warnings {
        println!(
            "{}: {:?}",
            w.message,
            &existing_text[(w.start.max(10) - 10)..(w.end + 10).min(existing_text.len())]
        );
    }
    assert!(warnings.is_empty());

    // let mut visitor = ColorVisitor::default();
    let mut visitor = ComboTableVisitor::default();
    visitor.set_base_text(existing_text);
    visit_nodes(&mut visitor, &parsed.nodes, existing_text);

    let replacements = visitor.replacements;
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

    out
}

async fn stuff() {
    let cred: Cred =
        serde_json::from_str(&std::fs::read_to_string("bot-creds.json").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.set_user_agent("dustloop botto (by moxian)");
    api.set_edit_delay(Some(100));
    api.login(cred.name, cred.password).await.unwrap();
    // let token = api.get_edit_token().await.unwrap();

    let existing_text = get_existing_page_text(&api, "GGST/Giovanna/Combos")
        .await
        .unwrap();
    // let existing_text = "===<big>{{clr|1|5P}}</big>===".to_string();
    // println!("{}", existing_text);
    let new_text = colorize_page(&existing_text);
    println!("{}", new_text)
}

#[tokio::main]
async fn main() {
    stuff().await;
}
