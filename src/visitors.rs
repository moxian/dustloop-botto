// use crate::parse;
use crate::parse::WikiVisitor;
use crate::prelude::*;
use parse_wiki_text::Node;
#[allow(unused_imports)]
use std::collections::{BTreeMap, BTreeSet};
pub struct ColorVisitor {
    config: ColorConfig,
    base_text: String,
    replacements: Vec<(String, std::ops::Range<usize>)>,
    errors: bool,
    seen: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    regex_cache: BTreeMap<String, regex::Regex>,
}
#[derive(serde::Deserialize)]
pub struct ColorConfig {
    moves: std::collections::BTreeMap<String, Vec<String>>,
    nonmoves: Vec<String>,
    // ok_colors: std::collections::BTreeMap<String, Vec<String>>,
    skip: BTreeSet<String>,
}
impl ColorVisitor {
    pub fn new(mut config: ColorConfig) -> Self {
        for moves in config.moves.values_mut() {
            moves.iter_mut().for_each(|m| *m = m.to_lowercase());
        }
        config.skip = config.skip.into_iter().map(|m| m.to_lowercase()).collect();
        config
            .nonmoves
            .iter_mut()
            .for_each(|m| *m = m.to_lowercase());
        ColorVisitor {
            config,
            base_text: Default::default(),
            replacements: Default::default(),
            errors: false,
            seen: Default::default(),
            regex_cache: Default::default(),
        }
    }

    fn get_regex(&self, letter_color: &str) -> &regex::Regex {
        self.regex_cache.get(letter_color).unwrap()
    }
}
fn make_regex(regex_cache: &mut BTreeMap<String, regex::Regex>, letter_color: &str) {
    if regex_cache.contains_key(letter_color) {
        return;
    }
    let prefixes = [
        "j.?", r"\d+", r"\[\d\]", r"/", r"bt\.", r"b\.", r"dj\.?", r"ws\.", r"hs\.", r"c\.",
        r"f\.", r"w\.", r"tj\.", "c", "f", r"dl\.", "delayed", "AA", "CH", "OTG",
    ];
    let mut letter_pat = vec![
        format!(r"{}", letter_color),
        format!(r"~{}", letter_color),
        format!(r"\[\d?{}\]", letter_color),
        format!(r"\]{}\[", letter_color),
        format!(r"\({}\)", letter_color),
        "".to_string() + r"\{" + letter_color + r"\}",
    ];
    if letter_color == "H" {
        letter_pat.push("HS".into());
    }
    let suffixes = [
        r"x\d",
        r"\(\d\)",
        r"\&",
        r"\*",
        r"~",
        r"<sub>\d</sub>",
        "whiff",
        "(w)",
    ];
    let pattern = format!(
        r"^(\s*({})*\s*({})+\s*({})*\s*([|>~])?\s*)+$",
        prefixes.join(r"\s*|\s*"),
        letter_pat.join(r"\s*|\s*"),
        suffixes.join(r"\s*|\s*")
    );
    let re = regex::RegexBuilder::new(&pattern)
        .case_insensitive(true)
        .build()
        .unwrap();
    regex_cache.insert(letter_color.to_string(), re);
}

impl WikiVisitor for ColorVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
    }
    fn get_replacements(&self) -> anyhow::Result<&[(String, std::ops::Range<usize>)]> {
        if self.errors {
            return Err(anyhow::anyhow!("errors encounterd"));
        }
        return Ok(self.replacements.as_slice());
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
                        let set_color = parameters[0].as_str(&self.base_text);
                        let letter_color = match set_color {
                            "1" | "P" => "P",
                            "2" | "K" => "K",
                            "3" | "S" => "S",
                            "4" | "H" => "H",
                            "5" | "D" => "D",
                            "6" | "7" | "8" => return, //leave it alone
                            "added" | "new" | "removed" | "reworked" | "buff" | "nerf" => return, // leave alone
                            _ => panic!("unknown color {:?}", set_color),
                        };
                        if ["P", "K", "S", "H", "D"].contains(&set_color) {
                            return;
                        }
                        if parameters.len() != 2 {
                            self.errors = true;
                            println!(
                                "Not a valid color usage: {:?}",
                                node.as_str(&self.base_text)
                            )
                        }
                        let colored_text_orig = parameters[1].as_str(&self.base_text);
                        let colored_text = colored_text_orig.to_lowercase();
                        if self.config.nonmoves.iter().any(|m| m == &colored_text) {
                            return;
                        }
                        if self.config.moves[letter_color]
                            .iter()
                            .any(|m| m == &colored_text)
                        {
                            // ok
                        } else if self.config.skip.contains(colored_text.as_str()) {
                            // ok for now
                        } else {
                            make_regex(&mut self.regex_cache, &letter_color);
                            let re = self.get_regex(&letter_color);
                            if !re.is_match(&colored_text) {
                                self.errors = true;
                                let slot = self
                                    .seen
                                    .entry(letter_color.to_string())
                                    .or_insert_with(|| Default::default());
                                if !slot.contains(&colored_text) {
                                    slot.insert(colored_text);
                                    println!("{}: {}", letter_color, colored_text_orig);
                                }
                                return;
                            } else {
                                // println!("{:?} matched {}", colored_text, pattern);
                            }
                        }
                        // println!(
                        //     "hi we are at around {:?}",
                        //     &existing_text[(node.start().max(10) - 10)
                        //         ..((node.end() + 10).min(existing_text.len()))]
                        // );
                        self.replacements.push((
                            letter_color.to_string(),
                            parameters[0].start..parameters[0].end,
                        ));
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
    replacements: Vec<(String, std::ops::Range<usize>)>,
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
        Ok(&self.replacements)
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
