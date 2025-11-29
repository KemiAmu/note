pub mod models {
    pub mod pages;
    pub mod types;
    pub mod users;
}

pub mod handlers {
    mod auth;
    mod home;
    mod page;
    mod user;
    pub use auth::*;
    pub use home::*;
    pub use page::*;
    pub use user::*;
}

pub mod config {
    use serde::Deserialize;
    use std::sync::LazyLock;

    #[derive(Deserialize)]
    pub struct Config {
        pub server_addr: &'static str,
        pub database_path: &'static str,
        pub site_root: &'static str,
        pub base_url: &'static str,
        pub cookie_path: &'static str,
        pub site_title: &'static str,
        pub secret_invite: &'static str,
        pub secret_passwd: &'static str,
    }

    pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
        let content = std::fs::read_to_string("server.toml").unwrap();
        let value: toml::Value = toml::from_str(&content).unwrap();
        let get = |key: &str| -> &'static str {
            Box::leak(value[key].as_str().unwrap().to_owned().into_boxed_str())
        };

        Config {
            server_addr: get("server_addr"),
            database_path: get("database_path"),
            site_root: get("site_root"),
            base_url: get("base_url"),
            cookie_path: get("cookie_path"),
            site_title: get("site_title"),
            secret_invite: get("secret_invite"),
            secret_passwd: get("secret_passwd"),
        }
    });
}

pub mod token {
    use base64::prelude::*;
    use std::sync::LazyLock;

    fn signature(claim: &str, secret: impl AsRef<[u8]>) -> String {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(secret);
        hasher.update(claim);
        BASE64_URL_SAFE_NO_PAD.encode(hasher.finalize())
    }

    pub static TOKEN_SECRET: LazyLock<[u8; 32]> = LazyLock::new(|| rand::random());

    pub struct Token;

    impl Token {
        pub fn new(sub: &str, age: i64, secret: impl AsRef<[u8]>) -> String {
            let now = time::UtcDateTime::now().unix_timestamp();
            let exp = now + age;

            let payload =
                BASE64_URL_SAFE_NO_PAD.encode([sub.as_bytes(), &exp.to_ne_bytes()].concat());
            let sign = signature(&payload, secret);
            format!("{payload}{sign}")
        }

        pub fn parse(token: &str, secret: impl AsRef<[u8]>) -> Option<String> {
            // SHA3-256 hash in base64 is always 43 bytes
            let (payload, sign) = token.split_at(token.len().checked_sub(43)?);

            if signature(payload, secret) != sign {
                return None;
            }
            let bytes = BASE64_URL_SAFE_NO_PAD.decode(payload).ok()?;
            let (sub_bytes, exp_bytes) =
                bytes.split_at(bytes.len().checked_sub(std::mem::size_of::<i64>())?);
            let exp = i64::from_ne_bytes(exp_bytes.try_into().ok()?);

            if exp <= time::UtcDateTime::now().unix_timestamp() {
                return None;
            }
            let sub = std::str::from_utf8(sub_bytes).ok()?;
            Some(sub.to_string())
        }
    }
}
