use crate::css::Selector::Simple;
use crate::css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};
use std::collections::HashMap;

// Map from CSS property names to values.
type PropertyMap = HashMap<String, Value>;

// A node with associated style data.
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

pub enum Display {
    Inline,
    Block,
    None,
}

impl<'a> StyledNode<'a> {
    /// Return the specified value of a property if it exists, otherwise `None`.
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    /// Return the specified value of property `name`, or property `fallback_name` if that doesn't
    /// exist, or value `default` if neither does.
    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    /// The value of the `display` property (defaults to inline).
    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match s.as_str() {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match selector {
        Simple(s) => matches_simple_selector(elem, s),
    }
}

fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }

    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    if selector
        .class
        .iter()
        .any(|class| elem.classes().contains(class.as_str()))
    {
        return false;
    }

    true
}

type MatchedRule<'a> = (Specificity, &'a Rule);

// If `rule` matched `elem`, return a `MatchedRule`. Otherwise return `None`.
fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    // Find the first (highest-specificity) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, selector))
        .map(|selector| (selector.specificity(), rule))
}

// Find all CSS rules that match the given element.
fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

// Apply styles to a single element, returning the specified values.
fn specified_values(elem: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    // Go through the rules from lowest to highest specificity
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

// Apply a stylesheet to an entire DOM tree, returning a StyledNode tree.
pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        specified_values: match root.node_type {
            NodeType::Text(_) => HashMap::new(),
            NodeType::Element(ref elem) => specified_values(elem, stylesheet),
        },
        children: root
            .children
            .iter()
            .map(|child| style_tree(child, stylesheet))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{Declaration, Unit};
    use crate::dom::{ElementData, Node, NodeType};

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
