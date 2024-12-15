use colored::*;

pub fn print_colored(text: &str, color: &str) {
    match color {
        "red" => println!("{}", text.red()),
        "green" => println!("{}", text.green()),
        "yellow" => println!("{}", text.yellow()),
        "blue" => println!("{}", text.blue()),
        "magenta" => println!("{}", text.magenta()),
        "cyan" => println!("{}", text.cyan()),
        "white" => println!("{}", text.white()),
        _ => println!("{}", text),
    }
}