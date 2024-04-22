use ego_tree::NodeRef;
use miette::Result;
use scraper::{node::Element, Html, Node};

pub trait DomVisitor<'dom> {
    fn visit_element(&mut self, node: &'dom Element);
}

#[derive(Debug)]
pub struct DomWalker {
    dom: Html,
}

impl DomWalker {
    pub fn new(html: &str) -> Result<Self> {
        let dom = Html::parse_document(html);
        Ok(Self { dom })
    }

    pub fn walk<'s, V: DomVisitor<'s>>(&'s self, visitor: &mut V) {
        walk_dom(visitor, &self.dom);
    }
}

fn walk_dom<'dom>(visitor: &mut impl DomVisitor<'dom>, dom: &'dom Html) {
    let root = dom.root_element();
    for child in root.children() {
        walk_node(visitor, child)
    }
}

fn walk_node<'dom>(visitor: &mut impl DomVisitor<'dom>, node: NodeRef<'dom, Node>) {
    match node.value() {
        // TODO: visit other node kinds as necessary
        Node::Element(element) => {
            visitor.visit_element(element);
            for child in node.children() {
                walk_node(visitor, child);
            }
        }
        _ => {}
    }
}
