use colored::*;

pub fn print_logo() {
    let logo = r#"
   ___  ___  _      __      __         _ _  _ 
  / _ \/ __|| |     \ \    / /        | | || |
 | |_| |\__ \| |      \ \/\/ /         | | || |
 |  _  /___||_|       \_/\_/        _  |_||_||_|
 |_| |_|(_)                  | |    | |     | | 
   _/ _ __ _   _             |_|____|_|_____| |_
  | || '__| | | |            |_____|_____|_____|
  | || |  | |_| |             _   _  __  _    
  |_||_|   \__, |           | | | |(_)/ _|   
            __/ |           | |_| | _ | |_    
           |___/             \__,_||_||_|     

"#;
    println!("{}", logo.cyan().bold());
    println!(
        "{}",
        "      Stellar Soroban Security & Formal Verification Suite"
            .white()
            .italic()
    );
    println!(
        "{}",
        "      v0.1.0 | Securing the Stellar Ecosystem".dimmed()
    );
    println!();
}
