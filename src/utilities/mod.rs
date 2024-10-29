use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::{thread_rng, Rng};

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Unable to hash password")
        .to_string()
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(hash).expect("Unable to parse hash");
    match argon2.verify_password(password.as_bytes(), &password_hash) {
        Ok(_i) => true,
        Err(_e) => false,
    }
}

pub fn generate_unique_id(length: u8) -> String {
    let mut id = String::new();
    let mut rng = thread_rng();
    let character_set: [char; 36] = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    ];
    let mut i: u8 = 0;
    while i < length {
        id.push(character_set[rng.gen_range(0..character_set.len())]);
        i += 1;
    }
    id
}
