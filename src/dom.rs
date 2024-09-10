type AttrMap = HashMap<String, String>;


struct Node {
    // data common to all nodes
    children: Vec<Node>,

    // data specific to each node type
    node_type: NodeType,
}

enum NodeType {
    Text(String),
    Element(ElementData)
}

struct ElementData {
    tag_name: String,
    attributes: AttrMap,
}


fn text(data: String) -> Node {
    Node { children: Vec::new(), node_type: NodeType::Text(data) }
}

pub fn elem(tag_name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData {
            tag_name,
            attrs,
        }),
    }
}