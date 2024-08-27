use std::ops::Deref;

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
use ego_tree::{NodeId, NodeRef};
use miette::Result;
use scraper::{element_ref::Text, node::Element, Html, Node};

pub trait DomVisitor<'dom> {
    fn visit_element(&mut self, node: ElementRef<'dom>);
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
        let root = self.dom.root_element();
        for child in root.children() {
            walk_node(visitor, child, &self.dom)
        }
    }
}

#[allow(clippy::single_match)]
fn walk_node<'dom>(
    visitor: &mut impl DomVisitor<'dom>,
    node: NodeRef<'dom, Node>,
    dom: &'dom Html,
) {
    match node.value() {
        // TODO: visit other node kinds as necessary
        Node::Element(element) => {
            let wrapper = ElementRef::new(element, node.id(), dom);
            visitor.visit_element(wrapper);
            for child in node.children() {
                walk_node(visitor, child, dom);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ElementRef<'d> {
    element: &'d Element,
    id: NodeId,
    dom: &'d Html,
}

impl Deref for ElementRef<'_> {
    type Target = Element;

    fn deref(&self) -> &Self::Target {
        self.element
    }
}
impl<'d> ElementRef<'d> {
    pub fn new(element: &'d Element, id: NodeId, dom: &'d Html) -> Self {
        Self { element, id, dom }
    }

    #[inline]
    pub fn attr(&self, attr: &str) -> Option<&'d str> {
        self.element.attr(attr)
    }

    /// Inner text
    pub fn text(&self) -> Text<'d> {
        scraper::ElementRef::wrap(self.dom.tree.get(self.id).unwrap())
            .unwrap()
            .text()
    }
}
