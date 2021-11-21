use kuchiki::NodeRef;

#[derive(Default, Debug)]
pub struct ArticleData {
    pub image: Option<String>,
    pub title: Option<String>,
}

const TITLE_PROPERTIES: &[&str] = &["og:title", "title", "twiter:title", "discord:title"];

const IMAGE_PROPERTIES: &[&str] = &["og:image", "image", "twiter:image", "discord:image"];

pub fn try_extract_data(node: &NodeRef) -> ArticleData {
    let mut p = ArticleData::default();
    for k in node.select("meta").unwrap() {
        let attrs = k.attributes.borrow();
        let prop = attrs.get("property").unwrap_or_default();
        if p.title.is_none() {
            p.title = TITLE_PROPERTIES
                .iter()
                .find(|title| prop == **title)
                .map(|_| attrs.get("content").map(|x| x.to_string()))
                .flatten();
        }
        if p.image.is_none() {
            p.image = IMAGE_PROPERTIES
                .iter()
                .find(|image| prop == **image)
                .map(|_| attrs.get("content").map(|x| x.to_string()))
                .flatten();
        }
    }
    p
}
