use crate::config::CONFIG;
use crate::models::types::{Ex, Result};
use crate::token::Token;
use redb::{Database, ReadableTable, TableDefinition};
use std::collections::BTreeSet;

/// user: UserData
pub const USERS: TableDefinition<&str, UserData> = TableDefinition::new("users");

#[derive(Debug, Clone)]
pub struct UserData {
    passwd: [u8; 32],
    pub collabs: BTreeSet<String>,
    pub files: BTreeSet<String>,
}

impl UserData {
    /// sign up a user
    pub fn sign_up(db: &Database, user: &str, passwd: &str, invite_code: &str) -> Result<()> {
        #[inline]
        fn validate_name(n: &str) -> bool {
            n.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                && (3..=24).contains(&n.len())
        }

        // check
        let Some(inviter) = Token::parse(invite_code, CONFIG.secret_invite) else {
            return Err(Ex::InvalidInvite);
        };
        if !validate_name(user) {
            return Err(Ex::InvalidUsername);
        }

        let mut user_data = Self::new(passwd);
        let write_txn = db.begin_write()?;
        let mut users_table = write_txn.open_table(USERS)?;

        // check user exists
        if users_table.get(user)?.is_some() {
            return Err(Ex::UserExists);
        }

        // connect node (except for root)
        if !inviter.is_empty() {
            let mut inviter_entry = users_table
                .get_mut(inviter.as_str())?
                .ok_or(Ex::InvalidInvite)?;
            let mut inviter_data = inviter_entry.value().clone();
            inviter_data.collabs.insert(user.to_string());
            inviter_entry.insert(inviter_data)?;
            user_data.collabs.insert(inviter);
        }

        users_table.insert(user, user_data)?;
        drop(users_table);
        write_txn.commit()?;
        println!("Signed up user: {}", user);
        Ok(())
    }

    /// parse an invite code, return inviter's profile url
    pub fn link_collab(db: &Database, user: &str, invite_code: &str) -> Result<String> {
        // check
        let Some(inviter) = Token::parse(invite_code, CONFIG.secret_invite) else {
            return Err(Ex::InvalidInvite);
        };
        if inviter.is_empty() || inviter == user {
            return Ok(format!("{}@{}", CONFIG.base_url, user));
        }

        let write_txn = db.begin_write()?;
        {
            // connect node
            let mut users_table = write_txn.open_table(USERS)?;
            let mut inviter_entry = users_table
                .get_mut(inviter.as_str())?
                .ok_or(Ex::InvalidInvite)?;
            let mut inviter_data = inviter_entry.value().clone();
            inviter_data.collabs.insert(user.to_string());
            inviter_entry.insert(&inviter_data)?;
        }
        write_txn.commit()?;
        Ok(Self::get_profile_url(&inviter))
    }

    pub fn new(passwd: &str) -> Self {
        Self {
            passwd: Self::hash_passwd(passwd),
            collabs: BTreeSet::new(),
            files: BTreeSet::new(),
        }
    }

    // password

    pub fn verify_passwd(&self, passwd: &str) -> Result<()> {
        match self.passwd == Self::hash_passwd(passwd) {
            true => Ok(()),
            false => Err(Ex::InvalidCredentials),
        }
    }
    pub fn update_passwd(&mut self, old_passwd: &str, passwd: &str) -> Result<()> {
        self.verify_passwd(old_passwd)?;
        self.passwd = Self::hash_passwd(passwd);
        Ok(())
    }
    fn hash_passwd(passwd: &str) -> [u8; 32] {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(CONFIG.secret_passwd);
        hasher.update(passwd);
        hasher.finalize().into()
    }

    // util

    pub fn get_profile_url(user: &str) -> String {
        format!("{}@{user}", CONFIG.base_url)
    }
}

impl redb::Value for UserData {
    type SelfType<'a> = UserData;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let (passwd, collabs, files) =
            <([u8; 32], Vec<String>, Vec<String>) as redb::Value>::from_bytes(data);
        UserData {
            passwd,
            collabs: BTreeSet::from_iter(collabs),
            files: BTreeSet::from_iter(files),
        }
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        <([u8; 32], Vec<String>, Vec<String>) as redb::Value>::as_bytes(&(
            value.passwd,
            Vec::from_iter(value.collabs.clone()),
            Vec::from_iter(value.files.clone()),
        ))
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("UserData")
    }
}
