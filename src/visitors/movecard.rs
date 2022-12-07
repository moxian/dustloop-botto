use crate::parse::WikiVisitor;
use crate::prelude::*;
use parse_wiki_text::Node;

#[derive(Default)]
pub struct MoveCardVisitor {
    base_text: String,
    replacements: Vec<(String, std::ops::Range<usize>)>,
    block_start: usize,
    descr_start: usize,
    is_ai: bool,
}
impl MoveCardVisitor {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WikiVisitor for MoveCardVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
    }
    fn get_replacements(&self) -> anyhow::Result<&[(String, std::ops::Range<usize>)]> {
        Ok(self.replacements.as_slice())
    }
    fn visit_start_tag(&mut self, node: &Node) {
        let node_str = node.as_str(&self.base_text);
        if node_str == r#"<div class="attack-container">"# {
            self.block_start = node.start();
            self.is_ai = false;
        } else if node_str == r#"<div class="attack-info">"# {
            self.is_ai = true;
        }
    }
    fn visit_heading(&mut self, node: &Node) {
        if node.as_str(&self.base_text) == "==== ====" {
            if self.is_ai {
                self.is_ai = false;
                self.descr_start = node.end();
            }
        }
    }
    fn visit_template(&mut self, node: &Node) {
        match node {
            Node::Template { name, .. } => {
                if name.as_str(&self.base_text) != "CloseCard" {
                    return;
                }
            }
            _ => return,
        }
        let block_start = self.block_start;
        let block_end = node.end();

        let descr_start = self.descr_start;
        let descr_end = node.start() - 1;
        // println!("{}\nxxxxxxxxx\n", &self.base_text[descr_start..descr_end]);
        let descr = self.base_text[descr_start..descr_end].trim();
        let mut out = String::new();
        out += "{{GGST Move Card\n|input=\n|description=\n";
        out += descr;
        out += "\n}}";
        self.replacements.push((out, block_start..block_end));
    }
}
