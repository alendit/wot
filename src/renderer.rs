use crate::model::{Outline, OutlineNode};

pub fn render_markdown(outline: &Outline) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(&outline.path.display().to_string());
    output.push('\n');

    render_nodes(&mut output, &outline.nodes, 0);
    output
}

fn render_nodes(output: &mut String, nodes: &[OutlineNode], depth: usize) {
    let indent = "  ".repeat(depth);

    for node in nodes {
        output.push_str(&indent);
        output.push_str("- ");
        output.push_str(&node.label);
        output.push_str(" [");
        output.push_str(&node.range.display());
        output.push_str("]\n");

        render_nodes(output, &node.children, depth + 1);
    }
}
