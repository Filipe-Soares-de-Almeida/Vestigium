/// The text of the first `<name>` element, as an event's `UserData` writes its fields.
pub(crate) fn element_text<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let start = xml.find(&open)? + open.len();
    let len = xml[start..].find('<')?;
    Some(xml[start..start + len].trim())
}

/// The text of `<Data Name='name'>` in an event's data, written with either quote style.
pub(crate) fn data_named<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    ['\'', '"'].into_iter().find_map(|quote| {
        let open = format!("<Data Name={quote}{name}{quote}>");
        let start = xml.find(&open)? + open.len();
        let len = xml[start..].find("</Data>")?;
        Some(xml[start..start + len].trim())
    })
}
