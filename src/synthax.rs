use kuchiki::NodeRef;

pub fn code_from_div(node: &NodeRef) -> String {
    let mut k = String::new();
    code_from_div_inner(node, &mut k);
    k
}

fn code_from_div_inner(node: &NodeRef, code: &mut String) {
    if let Some(e) = node.as_element() {
        let local = e.name.local.to_string();
        if let Some(e) = e.attributes.borrow().get("class") {
            if local == "div" {
                if e.contains("line") {
                    code.push_str(&node.text_contents());
                    code.push('\n');
                    return;
                }
            }
        }
        for i in node.children() {
            code_from_div_inner(&i, code);
        }
    } else if let Some(e) = node.as_text() {
        code.push_str(e.borrow().as_str());
    }
}
