use bcrypt::{hash, DEFAULT_COST};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <password>", args[0]);
        std::process::exit(1);
    }

    let password = &args[1];
    match hash(password, DEFAULT_COST) {
        Ok(hashed) => println!("{}", hashed),
        Err(e) => {
            eprintln!("Error hashing password: {}", e);
            std::process::exit(1);
        }
    }
}
