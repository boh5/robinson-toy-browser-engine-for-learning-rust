use std::collections::{HashMap, HashSet};

pub type AttrMap = HashMap<String, String>;

pub struct Node {
    // data common to all nodes
    pub children: Vec<Node>,

    // data specific to each node type
    pub node_type: NodeType,
}

impl Node {
    pub fn new(node_type: NodeType) -> Node {
        Node {
            node_type,
            children: Vec::new(),
        }
    }

    pub fn append_child(&mut self, node: Node) {
        self.children.push(node);
    }
}

pub enum NodeType {
    Text(String),
    Element(ElementData),
}

pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

impl ElementData {
    pub fn new(tag_name: &str, attributes: AttrMap) -> ElementData {
        ElementData {
            tag_name: tag_name.to_string(),
            attributes,
        }
    }

    pub fn id(&self) -> Option<&String> {
        self.attributes.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attributes.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new(),
        }
    }
}

pub fn text(data: String) -> Node {
    Node {
        children: Vec::new(),
        node_type: NodeType::Text(data),
    }
}

pub fn elem(tag_name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData {
            tag_name,
            attributes: attrs,
        }),
    }
}
