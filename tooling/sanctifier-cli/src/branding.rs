use colored::*;

pub fn print_logo() {
    let logo = r#"
   _____                       _     _   _  __  _                
  / ____|                     | |   | | (_)/ _|(_)               
 | (___   __ _  _ __    ___  | |_  | |  _ | |_  _   ___  _ __    
  \___ \ / _` || '_ \  / __| | __| | | | ||  _|| | / _ \| '__|   
  ____) | (_| || | | || (__  | |_  | | | || |  | ||  __/| |      
 |_____/ \__,_||_| |_| \___|  \__| |_| |_||_|  |_| \___||_|      
"#;
    println!("{}", logo.cyan().bold());
    println!("{}", "      Stellar Soroban Security & Formal Verification Suite".white().italic());
    println!();
}
