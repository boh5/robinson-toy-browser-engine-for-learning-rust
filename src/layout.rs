use crate::css::Unit::Px;
use crate::css::Value::{Keyword, Length};
use crate::layout::BoxType::{AnonymousBlock, BlockNode, InlineNode};
use crate::style::{Display, StyledNode};

#[derive(Clone, Copy, Default, Debug)]
struct Dimensions {
    // Position of the content area relative t the document origin:
    content: Rect,

    // Surrounding edges:
    padding: EdgeSizes,
    border: EdgeSizes,
    margin: EdgeSizes,
}

#[derive(Clone, Copy, Default, Debug)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Default, Debug)]
struct EdgeSizes {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

struct LayoutBox<'a> {
    dimensions: Dimensions,
    box_type: BoxType<'a>,
    children: Vec<LayoutBox<'a>>,
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type,
            dimensions: Default::default(), // initially set all fields to 0.0
            children: Vec::new(),
        }
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) | InlineNode(node) => node,
            AnonymousBlock => panic!("Anonymous block box has no style node"),
        }
    }
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(styled_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match styled_node.display() {
        Display::Block => BlockNode(styled_node),
        Display::Inline => InlineNode(styled_node),
        Display::None => panic!("Root node has display: none."),
    });

    // Create the descendant boxes.
    for child in &styled_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            Display::None => {} // Skip nodes with `display: none;`
        }
    }

    root
}

impl LayoutBox<'_> {
    /// Lay out a box and its descendants.
    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) => {}  // TODO
            AnonymousBlock => {} // TODO
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // child width can depend on parent width, so we need to calculate
        // this box's width before laying out its children.
        self.calculate_block_width(containing_block);

        // Determine where the box is located within its container.
        self.calculate_block_position(containing_block);

        // recursively lay out the children of this box.
        self.layout_block_children();

        // Parent height can depend on child height, so `calculate_height`
        // must be called *after* the children are laid out.
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = sum([
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ]
        .iter()
        .map(|v| v.to_px()));

        // if width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }
            if margin_right == auto {
                margin_right = Length(0.0, Px);
            }
        }

        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are overconstrained, calculate margin_riaght.
            (false, false, false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }
            // if exactly one size is auto, its used value follows from the equality.
            (false, false, true) => {
                margin_right = Length(underflow, Px);
            }
            (false, true, false) => {
                margin_left = Length(underflow, Px);
            }

            // If width is set to auto, any other auto values become 0.
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Length(0.0, Px);
                }
                if margin_right == auto {
                    margin_right = Length(0.0, Px);
                }

                if underflow >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Length(underflow, Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = Length(0.0, Px);
                    margin_right = Length(margin_right.to_px() + underflow, Px);
                }
            }
            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height
            + containing_block.content.y
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        for child in &mut self.children {
            child.layout(self.dimensions);
            // Increment the height so each child is laid out below the previous one.
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(Length(h, Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

    /// Where a new inline child should go.
    fn get_inline_container(&mut self) -> &mut Self {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            BlockNode(_) => {
                // If we've just generated an anonymous block box, keep using it.
                // Otherwise, create a new one.
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}
impl Dimensions {
    // The area covered by the content area plus its padding.
    fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    // The area covered by the content area plus padding and borders.
    fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    // The area covered by the content area plus padding, borders, and margin.
    fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

impl Rect {
    fn expanded_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

fn sum<I>(iter: I) -> f32
where
    I: Iterator<Item = f32>,
{
    iter.fold(0., |a, b| a + b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{Declaration, Rule, Selector, SimpleSelector, Stylesheet, Unit, Value};
    use crate::dom::{ElementData, Node, NodeType};
    use crate::style::style_tree;
    use std::collections::HashMap;

    #[test]
    fn style_tree_with_empty_stylesheet() {
        let root = Node::new(NodeType::Element(ElementData::new("div", HashMap::new())));
        let stylesheet = Stylesheet { rules: vec![] };
        let styled_node = style_tree(&root, &stylesheet);
        assert!(styled_node.specified_values.is_empty());
    }

    #[test]
    fn style_tree_with_single_rule() {
        let mut attributes = HashMap::new();
        attributes.insert("id".to_string(), "main".to_string());
        let root = Node::new(NodeType::Element(ElementData::new("div", attributes)));
        let rule = Rule {
            selectors: vec![Selector::Simple(SimpleSelector {
                tag_name: Some("div".to_string()),
                id: Some("main".to_string()),
                class: vec![],
            })],
            declarations: vec![Declaration {
                name: "color".to_string(),
                value: Value::Keyword("red".to_string()),
            }],
        };
        let stylesheet = Stylesheet { rules: vec![rule] };
        let styled_node = style_tree(&root, &stylesheet);
        assert_eq!(
            styled_node.specified_values.get("color"),
            Some(&Value::Keyword("red".to_string()))
        );
    }

    #[test]
    fn style_tree_with_nested_elements() {
        let mut root = Node::new(NodeType::Element(ElementData::new("div", HashMap::new())));
        let child = Node::new(NodeType::Element(ElementData::new("p", HashMap::new())));
        root.append_child(child);
        let rule = Rule {
            selectors: vec![Selector::Simple(SimpleSelector {
                tag_name: Some("p".to_string()),
                id: None,
                class: vec![],
            })],
            declarations: vec![Declaration {
                name: "margin".to_string(),
                value: Value::Length(10.0, Unit::Px),
            }],
        };
        let stylesheet = Stylesheet { rules: vec![rule] };
        let styled_node = style_tree(&root, &stylesheet);
        assert_eq!(
            styled_node.children[0].specified_values.get("margin"),
            Some(&Value::Length(10.0, Unit::Px))
        );
    }

    #[test]
    fn style_tree_with_text_node() {
        let root = Node::new(NodeType::Text("Hello".to_string()));
        let stylesheet = Stylesheet { rules: vec![] };
        let styled_node = style_tree(&root, &stylesheet);
        assert!(styled_node.specified_values.is_empty());
    }

    #[test]
    fn style_tree_with_conflicting_rules() {
        let root = Node::new(NodeType::Element(ElementData::new("div", HashMap::new())));
        let rule1 = Rule {
            selectors: vec![Selector::Simple(SimpleSelector {
                tag_name: Some("div".to_string()),
                id: None,
                class: vec![],
            })],
            declarations: vec![Declaration {
                name: "color".to_string(),
                value: Value::Keyword("red".to_string()),
            }],
        };
        let rule2 = Rule {
            selectors: vec![Selector::Simple(SimpleSelector {
                tag_name: Some("div".to_string()),
                id: None,
                class: vec![],
            })],
            declarations: vec![Declaration {
                name: "color".to_string(),
                value: Value::Keyword("blue".to_string()),
            }],
        };
        let stylesheet = Stylesheet {
            rules: vec![rule1, rule2],
        };
        let styled_node = style_tree(&root, &stylesheet);
        assert_eq!(
            styled_node.specified_values.get("color"),
            Some(&Value::Keyword("blue".to_string()))
        );
    }
}
