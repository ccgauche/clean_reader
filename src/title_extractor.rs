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
        for title in TITLE_PROPERTIES {
            if p.title.is_none() && prop == *title {
                p.title = attrs.get("content").map(|x| x.to_string());
            }
        }
        for image in IMAGE_PROPERTIES {
            if p.image.is_none() && prop == *image {
                p.image = attrs.get("content").map(|x| x.to_string());
            }
        }
    }
    p
}
