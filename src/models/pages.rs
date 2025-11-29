use redb::TableDefinition;

/// (user, file): PageData
pub const PAGES: TableDefinition<(&str, &str), PageData> = TableDefinition::new("pages");

// no fine-grained modification needed, so ownership doesn't matter
#[derive(Debug)]
pub struct PageData<'a> {
    pub title: &'a str,
    pub markdown: &'a str,
    pub html: &'a str,
    pub date: i64,
}

impl<'a> PageData<'a> {
    pub fn new(title: &'a str, markdown: &'a str, buf: &'a mut String) -> Self {
        // parse
        let parser = pulldown_cmark::Parser::new_ext(&markdown, pulldown_cmark::Options::all());
        pulldown_cmark::html::push_html(buf, parser);

        Self {
            title,
            markdown,
            html: buf.as_str(),
            date: time::UtcDateTime::now().unix_timestamp(),
        }
    }
}

impl<'a> redb::Value for PageData<'a> {
    type SelfType<'b>
        = PageData<'b>
    where
        'a: 'b;
    type AsBytes<'b>
        = Vec<u8>
    where
        Self: 'b;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'b>(data: &'b [u8]) -> Self::SelfType<'b>
    where
        Self: 'b,
    {
        let (title, markdown, html, date) =
            <(&str, &str, &str, i64) as redb::Value>::from_bytes(data);
        PageData {
            title,
            markdown,
            html,
            date,
        }
    }

    fn as_bytes<'b, 'c: 'b>(value: &'b Self::SelfType<'c>) -> Self::AsBytes<'b>
    where
        Self: 'c,
    {
        <(&str, &str, &str, i64) as redb::Value>::as_bytes(&(
            value.title,
            value.markdown,
            value.html,
            value.date,
        ))
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("PageData")
    }
}
