use crate::config::CONFIG;
use crate::models::pages::PAGES;
use crate::models::types::{AppState, Ex, Result};
use crate::models::users::USERS;
use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;
use redb::ReadableDatabase;

/// user page
pub async fn user_page(State(db): AppState, Path(user): Path<String>) -> Result<Html<String>> {
    #[derive(Template)]
    #[template(path = "user.html")]
    struct Page<'a> {
        base_url: &'a str,
        site_title: &'a str,
        username: &'a str,
        // [username]
        collabs: Vec<&'a String>,
        // (file, title)
        pages: Vec<(&'a str, &'a str)>,
    }

    let read_txn = db.begin_read()?;
    let users_table = read_txn.open_table(USERS)?;
    let pages_table = read_txn.open_table(PAGES).ok();
    let pages_table_ref = pages_table.as_ref();

    // user data
    let user_guard = users_table.get(user.as_str())?.ok_or(Ex::UserNotFound)?;
    let user_data = user_guard.value();

    // collabs list
    let collabs: Vec<&String> = user_data.collabs.iter().collect();

    // pages list
    let pages_guards: Vec<(&String, _)> = user_data
        .files
        .iter()
        .filter_map(|f| Some((f, pages_table_ref?.get(&(user.as_str(), f.as_str())).ok()??)))
        .collect();
    let pages: Vec<(&str, &str)> = pages_guards
        .iter()
        .map(|(file, guard)| (file.as_str(), guard.value().title))
        .collect();

    // render
    let page = Page {
        base_url: &CONFIG.base_url,
        site_title: &CONFIG.site_title,
        username: &user,
        collabs,
        pages,
    };
    Ok(Html(page.render()?))
}
