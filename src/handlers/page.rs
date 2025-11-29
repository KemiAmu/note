use crate::config::CONFIG;
use crate::handlers::auth::auth_component;
use crate::models::pages::{PAGES, PageData};
use crate::models::types::{AppState, Ex, Result};
use crate::models::users::USERS;
use askama::Template;
use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use redb::{ReadableDatabase, ReadableTable};

/// page view
pub async fn page_view(
    State(db): AppState,
    Path((user, file)): Path<(String, String)>,
) -> Result<Html<String>> {
    #[derive(Template)]
    #[template(path = "page.html")]
    struct Page<'a> {
        base_url: &'a str,
        site_title: &'a str,
        username: &'a str,
        file: &'a str,
        title: &'a str,
        content: &'a str,
        // (username, file, title)
        next_page: Option<(&'a str, &'a str, &'a str)>,
        date: &'a str,
    }

    let read_txn = db.begin_read()?;
    let pages_table = read_txn.open_table(PAGES)?;

    // get page and next page
    let mut page_iter = pages_table.range((user.as_str(), file.as_str())..)?;
    let current_page = page_iter.next().ok_or(Ex::PageNotFound)??.1;
    let current_page = current_page.value();
    let next_page = page_iter.next().transpose()?;
    let next_page = next_page
        .as_ref()
        .map(|(k, v)| (k.value().0, k.value().1, v.value().title));

    // render
    let page = Page {
        base_url: CONFIG.base_url,
        site_title: CONFIG.site_title,
        username: &user,
        file: &file,
        title: current_page.title,
        content: current_page.html,
        next_page: next_page,
        date: &time::UtcDateTime::from_unix_timestamp(current_page.date)
            .map_err(|_| Ex::InvalidTimestamp)?
            .format(&time::format_description::well_known::Iso8601::DATE)
            .map_err(|_| Ex::InvalidTimestamp)?,
    };
    Ok(Html(page.render()?))
}

/// page editor
pub async fn page_editor(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
    Path((user, file)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    #[derive(Template)]
    #[template(path = "edit.html")]
    struct Page<'a> {
        base_url: &'a str,
        site_title: &'a str,
        username: &'a str,
        file: &'a str,
        title: &'a str,
        markdown: &'a str,
    }

    // auth page
    let Some(auth_user) = auth else {
        let url = format!("{}@{user}/{file}", CONFIG.base_url);
        return Ok((StatusCode::FORBIDDEN, auth_component(None, &url)?).into_response());
    };

    let read_txn = db.begin_read()?;
    let users_table = read_txn.open_table(USERS)?;
    let pages_table = read_txn.open_table(PAGES)?;

    // check permissions
    if auth_user != user
        && let Some(user) = users_table.get(user.as_str())?
        && !user.value().collabs.contains(&auth_user)
    {
        return Err(Ex::PermissionDenied);
    }

    // target page
    let page = pages_table
        .get(&(user.as_str(), file.as_str()))?
        .ok_or(Ex::PageNotFound)?;
    let page = page.value();

    // render
    let page = Page {
        base_url: CONFIG.base_url,
        site_title: CONFIG.site_title,
        username: &user,
        file: &file,
        title: page.title,
        markdown: page.markdown,
    };
    Ok(Html(page.render()?).into_response())
}

/// api: update page
pub async fn page_update(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
    Path((user, file)): Path<(String, String)>,
    Json((title, markdown)): Json<(String, String)>,
) -> Result<()> {
    // check
    let Some(auth_user) = auth else {
        return Err(Ex::PermissionDenied);
    };

    let write_txn = db.begin_write()?;
    {
        let mut users_table = write_txn.open_table(USERS)?;
        let mut pages_table = write_txn.open_table(PAGES)?;

        // target user
        let mut target_entry = users_table
            .get_mut(user.as_str())?
            .ok_or(Ex::UserNotFound)?;
        let mut target_data = target_entry.value().clone();

        // check permissions
        if auth_user != user && !target_data.collabs.contains(&auth_user) {
            return Err(Ex::PermissionDenied);
        }

        // update file
        let mut page_entry = pages_table
            .get_mut(&(user.as_str(), file.as_str()))?
            .ok_or(Ex::PageNotFound)?;
        page_entry.insert(PageData::new(&title, &markdown, &mut String::new()))?;
        target_data.files.insert(file.clone());
        target_entry.insert(target_data)?;
    }
    write_txn.commit()?;
    println!("Updated page: @{}/{}", user, file);
    Ok(())
}

/// api: create page
pub async fn page_create(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
    Path((user, file)): Path<(String, String)>,
) -> Result<()> {
    #[inline]
    fn validate_name(n: &str) -> bool {
        n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            && (1..=120).contains(&n.len())
    }

    // check
    let Some(auth_user) = auth else {
        return Err(Ex::PermissionDenied);
    };
    if !validate_name(&file) {
        return Err(Ex::InvalidFilename);
    }

    let write_txn = db.begin_write()?;
    {
        let mut users_table = write_txn.open_table(USERS)?;
        let mut pages_table = write_txn.open_table(PAGES)?;

        // target user
        let mut target_entry = users_table
            .get_mut(user.as_str())?
            .ok_or(Ex::UserNotFound)?;
        let mut target_data = target_entry.value().clone();

        // check permissions
        if auth_user != user && !target_data.collabs.contains(&auth_user) {
            return Err(Ex::PermissionDenied);
        }

        // check if page already exists
        if pages_table.get(&(user.as_str(), file.as_str()))?.is_some() {
            return Err(Ex::PageAlreadyExists);
        }

        // create new page
        target_data.files.insert(file.clone());
        target_entry.insert(target_data)?;
        pages_table.insert(
            &(user.as_str(), file.as_str()),
            PageData::new("Untitled", "", &mut String::new()),
        )?;
    }
    write_txn.commit()?;
    println!("Created page: @{}/{}", user, file);
    Ok(())
}

/// api: delete page
pub async fn page_delete(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
    Path((user, file)): Path<(String, String)>,
) -> Result<()> {
    // check token
    let Some(auth_user) = auth else {
        return Err(Ex::PermissionDenied);
    };

    let write_txn = db.begin_write()?;
    {
        let mut users_table = write_txn.open_table(USERS)?;
        let mut pages_table = write_txn.open_table(PAGES)?;

        // target user
        let mut target_entry = users_table
            .get_mut(user.as_str())?
            .ok_or(Ex::UserNotFound)?;
        let mut target_data = target_entry.value().clone();

        // check permissions
        if auth_user != user && !target_data.collabs.contains(&auth_user) {
            return Err(Ex::PermissionDenied);
        }

        // remove
        target_data.files.remove(&file);
        target_entry.insert(target_data)?;
        pages_table.remove(&(user.as_str(), file.as_str()))?;
    }
    write_txn.commit()?;
    println!("Deleted page: @{}/{}", user, file);
    Ok(())
}
