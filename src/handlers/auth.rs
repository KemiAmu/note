use crate::config::CONFIG;
use crate::models::types::{AppState, Ex, Result};
use crate::models::users::{USERS, UserData};
use crate::token::{TOKEN_SECRET, Token};
use askama::Template;
use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use redb::ReadableDatabase;

/// issue a token cookie
fn issue_token_cookie(jar: CookieJar, user: Option<&str>) -> CookieJar {
    let token = user.map(|user| Token::new(user, 10324800, &TOKEN_SECRET.as_ref()));
    let cookie = Cookie::build(("token", token.unwrap_or_default()))
        .path(CONFIG.cookie_path)
        .max_age(time::Duration::seconds(user.map_or(0, |_| 10324800)))
        .secure(true)
        .http_only(true);
    if let Some(user) = user {
        println!("Issuing token cookie for user: {}", user);
    }
    jar.add(cookie)
}

/// auth page component
pub(crate) fn auth_component(invite_code: Option<&str>, prev_url: &str) -> Result<Html<String>> {
    #[derive(Template)]
    #[template(path = "auth.html")]
    struct Page<'a> {
        base_url: &'a str,
        site_title: &'a str,
        invite_code: Option<&'a str>,
        prev_url: &'a str,
    }

    // verify invite_code
    if let Some(invite) = invite_code
        && Token::parse(invite, CONFIG.secret_invite).is_none()
    {
        return Err(Ex::InvalidInvite);
    }

    // render html
    let page = Page {
        base_url: CONFIG.base_url,
        site_title: CONFIG.site_title,
        invite_code,
        prev_url,
    };
    Ok(Html(page.render()?))
}

/// sign in or redirect to profile
pub async fn auth_page(Extension(auth): Extension<Option<String>>) -> Result<Response> {
    match auth {
        None => Ok(auth_component(None, CONFIG.base_url).into_response()),
        Some(_username) => Ok(Redirect::to(CONFIG.base_url).into_response()),
    }
}

/// sign up handler
pub async fn sign_up_handler(
    State(db): AppState,
    jar: CookieJar,
    Json((user, passwd, invite_code)): Json<(String, String, String)>,
) -> Result<impl IntoResponse> {
    // sign up
    UserData::sign_up(&db, &user, &passwd, &invite_code)?;
    // issue token
    Ok(issue_token_cookie(jar, Some(&user)))
}

/// sign in handler
pub async fn sign_in_handler(
    State(db): AppState,
    jar: CookieJar,
    Json((user, passwd)): Json<(String, String)>,
) -> Result<impl IntoResponse> {
    // verify password
    let read_txn = db.begin_read()?;
    read_txn
        .open_table(USERS)?
        .get(user.as_str())?
        .ok_or(Ex::InvalidCredentials)?
        .value()
        .verify_passwd(&passwd)?;

    // issue token
    Ok(issue_token_cookie(jar, Some(&user)))
}

/// sign out (only cookie, no page)
pub async fn sign_out_handler(jar: CookieJar) -> impl IntoResponse {
    // issue a invalid token
    issue_token_cookie(jar, None)
}

/// visit an invitation
pub async fn invite_handler(
    State(db): AppState,
    Extension(auth): Extension<Option<String>>,
    Path(invite_code): Path<String>,
) -> Result<Response> {
    // sign in or sign up
    let Some(username) = auth else {
        let html = auth_component(Some(&invite_code), CONFIG.base_url);
        return Ok((StatusCode::FORBIDDEN, html).into_response());
    };

    // redirect to (inviter's) profile
    let profile_url = UserData::link_collab(&db, &username, &invite_code)?;
    Ok(Redirect::to(&profile_url).into_response())
}
