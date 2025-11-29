use crate::config::CONFIG;
use crate::models::pages::PAGES;
use crate::models::types::{AppState, Result};
use crate::token::Token;
use askama::Template;
use axum::extract::{Extension, State};
use axum::response::Html;
use redb::{ReadableDatabase, ReadableTable};

/// home page & work space
pub async fn home_page(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
) -> Result<Html<String>> {
    #[derive(Template)]
    #[template(path = "home.html")]
    struct Page<'a> {
        base_url: &'a str,
        site_title: &'a str,
        // [(username, file, title, date)]
        pages: Vec<(String, String, String)>,
        // (username, invite_code)
        user: Option<(String, String)>,
    }

    let read_txn = db.begin_read()?;

    let pages = if let Ok(pages_table) = read_txn.open_table(PAGES) {
        let i = pages_table.iter()?;
        i.filter_map(|result| {
            result.ok().map(|(key, value)| {
                let (user, file) = key.value();
                (user.into(), file.into(), value.value().title.into())
            })
        })
        .collect()
    } else {
        vec![]
    };
    // pages.sort_by(|a, b| b.3.cmp(&a.3));

    let user = auth.map(|username| {
        let t = Token::new(&username, 604800, CONFIG.secret_invite);
        (username, t)
    });

    let page = Page {
        base_url: &CONFIG.base_url,
        site_title: &CONFIG.site_title,
        pages,
        user,
    };
    Ok(Html(page.render()?))
}
