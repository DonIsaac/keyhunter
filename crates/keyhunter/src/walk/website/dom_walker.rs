/// Copyright © 2024 Don Isaac
///
/// This file is part of KeyHunter.
///
/// KeyHunter is free software: you can redistribute it and/or modify it
/// under the terms of the GNU General Public License as published by the Free
/// Software Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
/// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
/// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
/// more details.
///
/// You should have received a copy of the GNU General Public License along with
/// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
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
