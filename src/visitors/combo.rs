use crate::parse::WikiVisitor;
use crate::prelude::*;
use parse_wiki_text::Node;

#[derive(Default)]
pub struct ComboTableVisitor {
    base_text: String,
    in_table: bool,
    skip_table: bool,
    column_order: Option<Vec<String>>,
    replacements: Vec<(String, std::ops::Range<usize>)>,
    errors: bool,
}
impl ComboTableVisitor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WikiVisitor for ComboTableVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
    }
    fn get_replacements(&self) -> anyhow::Result<&[(String, std::ops::Range<usize>)]> {
        if self.errors {
            anyhow::bail!("Failed to do the thing. See errors above.");
        }
        Ok(&self.replacements)
    }
    fn visit_table_start(&mut self, _: &Node) {
        assert!(!self.in_table);
        self.in_table = true;
        self.skip_table = false;
        self.column_order = None;
    }
    fn visit_table_end(&mut self, _: &Node) {
        assert!(self.in_table);
        self.skip_table = false;
        self.in_table = false;
    }
    fn visit_table_row(&mut self, row: &parse_wiki_text::TableRow) {
        if row.cells.len() == 0 {
            //either the last row OR templatized already
            return;
        }
        if row.cells.len() <= 3 || self.skip_table {
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
            "recipePS",
            // "checkedVersion",
        ];

        let mut out = String::new();
        if self.column_order.is_none() {
            // header
            let mut order = vec![];
            // must be the heading
            assert!(row
                .cells
                .iter()
                .all(|c| c.type_ == parse_wiki_text::TableCellType::Heading));
            for cell in &row.cells {
                let caption = cell.text_content(&self.base_text).to_lowercase();

                order.push(match caption.as_str() {
                    "combo" => "combo",
                    "position" => "position",
                    "damage" => "damage",
                    "tension gain" => "tensionGain",
                    "works on:" | "works on" => "worksOn",
                    "difficulty" => "difficulty",
                    "video" | "video demonstration" => "video",
                    "notes" => "notes",
                    "recipe" | "combo recipe no." | "recipe id" | "recipes (pc)"
                    | "combo recipe (pc)" => "recipePC",
                    "notation" => {
                        self.skip_table = true;
                        return;
                    }
                    z => panic!("unknown caption {:?}", z),
                });
                if let Some(a) = &cell.attributes {
                    if caption.as_str() == "recipe"
                        && a.len() == 1
                        && a[0].as_str(&self.base_text).to_lowercase() == "colspan=2"
                    {
                        order.push("recipePS");
                    }
                }
            }
            // assert_eq!(
            //     row.cells.len(),
            //     order.len(),
            //     "{:?} vs\n{:?}",
            //     row.as_str(&self.base_text),
            //     order,
            // );
            self.column_order = Some(order.into_iter().map(|x| x.to_string()).collect());

            out += "|-\n{{GGST-ComboTableHeader}}"
        } else {
            // non-header
            if row.cells.len() != self.column_order.as_ref().unwrap().len() {
                println!(
                    "row length mismatch: {} vs {}: {}",
                    row.cells.len(),
                    self.column_order.as_ref().unwrap().len(),
                    row.as_str(&self.base_text)
                );
                self.errors = true;
                return;
            }

            let mut kvs = std::collections::BTreeMap::new();
            // kvs.insert("checkedVersion", "");

            for (cell, column) in row.cells.iter().zip(self.column_order.as_ref().unwrap()) {
                kvs.insert(column.as_str(), cell.text_content(&self.base_text));
            }

            out += "|-\n{{GGST-ComboTableRow\n";
            for col in desired_order {
                let mut val = kvs.remove(col);
                if ["video", "recipePC", "recipePS"].contains(&col) {
                    val = Some(val.unwrap_or(""));
                }
                if let Some(mut val) = val {
                    val = val.trim();
                    if ["video", "recipePC", "recipePS"].contains(&col) && val == "-" {
                        // continue;
                        val = ""
                    }
                    if val == "" {
                        // continue;
                    }

                    out += &format!("|{} = {}\n", col, val)
                }
            }
            out += &"|checkedVersion = \n";
            out += "}}";
            assert!(kvs.is_empty(), "Some keys left over! {:?}", kvs);
        }

        self.replacements.push((out, row.start..row.end));
    }
}
