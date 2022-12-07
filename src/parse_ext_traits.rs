use parse_wiki_text::Node;

pub trait NodeExt {
    fn as_str<'a>(&self, base_text: &'a str) -> &'a str;
    fn range(&self) -> std::ops::Range<usize>;
    fn start(&self) -> usize;
    fn end(&self) -> usize;
}
impl NodeExt for parse_wiki_text::Node<'_> {
    fn as_str<'a>(&self, base_text: &'a str) -> &'a str {
        &base_text[self.range()]
    }
    fn range(&self) -> std::ops::Range<usize> {
        match self {
            Node::Text { start, end, .. }
            | Node::Link { start, end, .. }
            | Node::Template { start, end, .. }
            | Node::Comment { start, end, .. }
            | Node::StartTag { start, end, .. }
            | Node::Bold { start, end, .. }
            | Node::EndTag { start, end, .. }
            | Node::Heading { start, end, .. } => *start..*end,

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

pub trait ParameterExt {
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

pub trait TableCellExt {
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

pub trait TableRowExt {
    fn as_str<'a>(&self, source: &'a str) -> &'a str;
}
impl TableRowExt for parse_wiki_text::TableRow<'_> {
    fn as_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

pub trait VecNodeExt {
    fn as_str<'a>(&self, source: &'a str) -> &'a str;
}
impl VecNodeExt for Vec<Node<'_>> {
    fn as_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.first().unwrap().start()..self.last().unwrap().end()]
    }
}
