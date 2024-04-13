
use miette::{IntoDiagnostic as _, Result};
use html_parser::{Dom, Element, Node};

pub trait DomVisitor<'dom> {
    fn visit_element(&mut self, node: &'dom Element);
}

#[derive(Debug)]
pub struct DomWalker {
    dom: Dom
}

impl DomWalker {
    pub fn new(html: &str) -> Result<Self> {
        let dom = Dom::parse(html).into_diagnostic()?;
        Ok(Self { dom })
    }

    pub fn walk<'s, V: DomVisitor<'s>>(&'s self, visitor: &mut V) {
        walk_dom(visitor, &self.dom);
    }
}

fn walk_dom<'dom>(visitor: &mut impl DomVisitor<'dom>, dom: &'dom Dom) {
    for child in &dom.children {
        walk_node(visitor, child)
    }
}

fn walk_node<'dom>(visitor: &mut impl DomVisitor<'dom>, node: &'dom Node) {

        match node {
            // TODO: visit other node kinds as necessary
            Node::Element(element) => {
                visitor.visit_element(&element);
                for child in &element.children {
                    walk_node(visitor, child);
                }
            },
            _ => {}
        }
}
