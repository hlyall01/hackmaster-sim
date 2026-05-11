pub(crate) struct StaticAsset {
    pub(crate) content_type: &'static str,
    pub(crate) body: &'static str,
}

pub(crate) const INDEX_HTML: &str = include_str!("web/index.html");

pub(crate) fn get(path: &str) -> Option<StaticAsset> {
    match path {
        "/static/styles.css" => Some(StaticAsset {
            content_type: "text/css; charset=utf-8",
            body: include_str!("web/styles.css"),
        }),
        "/static/js/main.js" => Some(js(include_str!("web/js/main.js"))),
        _ => None,
    }
}

fn js(body: &'static str) -> StaticAsset {
    StaticAsset {
        content_type: "text/javascript; charset=utf-8",
        body,
    }
}
