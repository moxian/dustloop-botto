// use crate::parse;
use parse_wiki_text::Node;
use crate::parse::WikiVisitor;
use crate::prelude::*;

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
pub struct ComboTableVisitor {
    base_text: String,
    in_table: bool,
    column_order: Option<Vec<String>>,
    pub replacements: Vec<(String, std::ops::Range<usize>)>,
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
        if row.cells.len() <= 3 {
            // not a combo
            return;
        }
        let desired_order = [
            "combo",
            "position",
            "damage",
            "tensionGain",
            "worksOn",
            "difficulty",
            "notes",
            "video",
            "recipePC",
        ];

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
                    "recipe" => "recipePC",
                    _ => panic!("unknown caption {:?}", caption),
                });
            }
            self.column_order = Some(order.into_iter().map(|x| x.to_string()).collect());
            assert_eq!(
                row.cells.len(),
                self.column_order.as_ref().unwrap().len(),
                "{:?} vs\n{:?}",
                row.as_str(&self.base_text),
                self.column_order
            );
            out += "|-\n{{GGST-ComboTableHeader}}"
        } else {
            assert_eq!(
                row.cells.len(),
                self.column_order.as_ref().unwrap().len(),
                "{:?}",
                row.as_str(&self.base_text)
            );
            let mut kvs = std::collections::BTreeMap::new();

            for (cell, column) in row.cells.iter().zip(self.column_order.as_ref().unwrap()) {
                kvs.insert(column.as_str(), cell.text_content(&self.base_text));
            }

            out += "|-\n{{GGST-ComboTableRow\n";
            for col in desired_order {
                let val = kvs.remove(col);
                if let Some(mut val) = val {
                    val = val.trim();
                    if col == "video" && val == "-" {
                        continue;
                    }
                    if val == "" {
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
