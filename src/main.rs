use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Router, middleware};
use axum_extra::extract::cookie::CookieJar;
use redb::Database;
use std::error::Error;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

use note::config::CONFIG;
use note::handlers::*;
use note::token::{TOKEN_SECRET, Token};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(CONFIG.server_addr).await.unwrap();
    let db = Database::create(CONFIG.database_path)?;

    let root_invite = Token::new("", 900, CONFIG.secret_invite);
    println!("Root invite code: {}invite/{root_invite}", CONFIG.base_url);

    // home page & work space
    let app = Router::new().route("/", get(home_page));

    let app = app // auth
        .route("/auth", get(auth_page)) // html or redirect
        .route("/auth/", get(auth_page)) // html or redirect
        .route("/auth/sign-out", get(sign_out_handler)) // [] -> cookie
        .route("/auth/sign-out/", get(sign_out_handler)) // [] -> cookie
        .route("/auth/sign-in", post(sign_in_handler)) // [user, passwd] -> cookie
        .route("/auth/sign-in/", post(sign_in_handler)) // [user, passwd] -> cookie
        .route("/auth/sign-up", post(sign_up_handler)) // [user, passwd, invite_code] -> cookie
        .route("/auth/sign-up/", post(sign_up_handler)) // [user, passwd, invite_code] -> cookie
        .route("/invite/{invite_code}", get(invite_handler)) // html or redirect
        .route("/invite/{invite_code}/", get(invite_handler)); // html or redirect

    let app = app // user
        .route("/@{user}", get(user_page)) // html
        .route("/@{user}/", get(user_page)); // html

    let app = app // page
        .route("/@{user}/{page}", get(page_view)) // html
        .route("/@{user}/{page}/", get(page_view)) // html
        .route("/@{user}/{page}/edit", get(page_editor)) // html
        .route("/@{user}/{page}/edit/", get(page_editor)) // html
        .route("/page/{user}/{page}", put(page_create)) // [] -> ok
        .route("/page/{user}/{page}/", put(page_create)) // [] -> ok
        .route("/page/{user}/{page}", post(page_update)) // [title, markdown] -> ok
        .route("/page/{user}/{page}/", post(page_update)) // [title, markdown] -> ok
        .route("/page/{user}/{page}", delete(page_delete)) // [] -> ok
        .route("/page/{user}/{page}/", delete(page_delete)); // [] -> ok

    let app = app
        .fallback_service(ServeDir::new(CONFIG.site_root))
        .layer(middleware::from_fn(auth_middleware))
        .layer(CompressionLayer::new().zstd(true).gzip(true).deflate(true))
        .with_state(Arc::new(db));

    axum::serve(listener, app).await.unwrap();
    Ok(())
}

pub async fn auth_middleware(jar: CookieJar, mut request: Request, next: Next) -> Response {
    let auth: Option<String> = jar
        .get("token")
        .and_then(|cookie| Token::parse(cookie.value(), &TOKEN_SECRET.as_ref()));

    request.extensions_mut().insert(auth);
    next.run(request).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_json_filters() {
        use askama::Template;
        #[derive(Template)]
        #[template(source = "\"{{data|json}}\"\n\"{{data|json|safe}}\"", ext = "html")]
        struct Page<'a> {
            data: &'a str,
        }

        let test_data = r#"{"key": "value", "special": "<script>alert('xss')</script>"}"#;
        let json_output = Page { data: test_data }.render().unwrap();
        println!("{json_output}");
    }
}
