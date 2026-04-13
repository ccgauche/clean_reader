use markup5ever_rcdom::{Handle, NodeData};

#[derive(Default, Debug, Clone)]
pub struct ArticleData {
    pub image: Option<String>,
    pub title: Option<String>,
    pub etitle: Option<String>,
}

const TITLE_PROPERTIES: &[&str] = &["og:title", "title", "twiter:title", "discord:title"];

const IMAGE_PROPERTIES: &[&str] = &["og:image", "image", "twiter:image", "discord:image"];

pub fn try_extract_data(root: &Handle) -> ArticleData {
    let mut p = ArticleData {
        etitle: find_first_by_tag(root, "title").map(|n| text_contents(&n)),
        ..Default::default()
    };
    visit(root, &mut |node| {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            if name.local.as_ref() != "meta" {
                return;
            }
            let attrs = attrs.borrow();
            let prop = attrs
                .iter()
                .find(|a| a.name.local.as_ref() == "property")
                .map(|a| a.value.to_string())
                .unwrap_or_default();
            let content = || {
                attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "content")
                    .map(|a| a.value.to_string())
            };
            if p.title.is_none() && TITLE_PROPERTIES.iter().any(|t| prop == *t) {
                p.title = content();
            }
            if p.image.is_none() && IMAGE_PROPERTIES.iter().any(|t| prop == *t) {
                p.image = content();
            }
        }
    });
    p
}

fn find_first_by_tag(root: &Handle, tag: &str) -> Option<Handle> {
    if let NodeData::Element { name, .. } = &root.data {
        if name.local.as_ref() == tag {
            return Some(root.clone());
        }
    }
    for child in root.children.borrow().iter() {
        if let Some(found) = find_first_by_tag(child, tag) {
            return Some(found);
        }
    }
    None
}

fn text_contents(node: &Handle) -> String {
    let mut out = String::new();
    fn inner(node: &Handle, out: &mut String) {
        if let NodeData::Text { contents } = &node.data {
            out.push_str(&contents.borrow());
        }
        for child in node.children.borrow().iter() {
            inner(child, out);
        }
    }
    inner(node, &mut out);
    out
}

fn visit(node: &Handle, f: &mut dyn FnMut(&Handle)) {
    f(node);
    for child in node.children.borrow().iter() {
        visit(child, f);
    }
}
