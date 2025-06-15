use argon2::password_hash::{Error, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::random_range;
use rand_core::OsRng;
use crate::error::AnyErr;

pub fn hash(password: &str) -> Result<String, AnyErr> {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify(password: &str, password_hash: &str) -> Result<bool, AnyErr> {
    let password_hash = PasswordHash::new(password_hash)?;

    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &password_hash) {
        Ok(_) => Ok(true),
        Err(Error::Password) => Ok(false),
        Err(err) => Err(err.into()),
    }
}

const SESSION_CHARS: [char; 62] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
pub fn random_session() -> String {
    let mut s = String::new();
    for _ in 0..128 {
        s.push(SESSION_CHARS[random_range(0..SESSION_CHARS.len())]);
    }
    s
}