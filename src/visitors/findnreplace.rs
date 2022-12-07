use crate::parse::WikiVisitor;

pub struct FindReplaceVisitor {
    base_text: String,
    config: FindReplaceConfig,
    replacements: Vec<(String, std::ops::Range<usize>)>,
    // errors: bool,
}
impl FindReplaceVisitor {
    pub fn new(config: &FindReplaceConfig) -> Self {
        Self {
            config: config.clone(),
            base_text: Default::default(),
            replacements: Default::default(),
        }
    }
    fn gen_replacements(&mut self) {
        for pred in &self.config.predicates {
            if !self.base_text.contains(pred) {
                return; // don't change anything
            }
        }
        for (find, replace) in &self.config.re_patterns {
            let re = regex::Regex::new(find).unwrap();
            for cap in re.captures_iter(&self.base_text) {
                let m = cap.get(0).expect("capture 0 must always exist wtf");
                // let fragment = m.as_str().to_string();
                let mut fragment = String::new();
                cap.expand(replace, &mut fragment);
                self.replacements.push((fragment, m.range()));
                // println!("{:?}", self.replacements.last().unwrap());
            }
        }
        for (find, replace) in &self.config.plain_patterns {
            let found = self.base_text.find(find);
            let found = match found {
                Some(f) => f,
                None => continue,
            };
            self.replacements
                .push((replace.clone(), found..(found + find.len())))
        }
    }
}
#[derive(serde::Deserialize, Clone)]
pub struct FindReplaceConfig {
    predicates: Vec<String>,
    re_patterns: Vec<(String, String)>,
    plain_patterns: Vec<(String, String)>,
}
impl FindReplaceConfig {}

impl WikiVisitor for FindReplaceVisitor {
    fn set_base_text(&mut self, base_text: &str) {
        self.base_text = base_text.to_string();
        self.gen_replacements();
    }
    fn get_replacements(&self) -> anyhow::Result<&[(String, std::ops::Range<usize>)]> {
        Ok(&self.replacements)
    }
}
