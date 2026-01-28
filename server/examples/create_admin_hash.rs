use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let password = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "admin123".to_string());

    let hashed = hash(password.as_bytes(), DEFAULT_COST).expect("Failed to hash password");
    println!("Password: {}", password);
    println!("Bcrypt hash: {}", hashed);
    println!();
    println!("SQL UPDATE command:");
    println!(
        "UPDATE admin_users SET password_hash = '{}' WHERE username = 'admin';",
        hashed
    );
}
