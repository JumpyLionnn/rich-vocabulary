use std::io::{self, Write};



pub fn input(prompt: &str) -> io::Result<String> {
    let mut line = String::new();
    print!("{prompt}");
    io::stdout().flush()?;
    io::stdin().read_line(&mut line)?;
    Ok(line)
}

pub fn str_to_bool(mut str: String) -> Option<bool> {
    str.make_ascii_lowercase();
    match str.trim() {
        "y" | "yes" | "yeah" | "yea" | "true" | "on" => Some(true),
        "n" | "no" | "nope" | "false" | "off" => Some(false),
        _ => None,
    }
}