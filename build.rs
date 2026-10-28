//! Copies src/shader.frag into the build with its comments and indentation taken out, so
//! the page downloads less. Every line stays on its own line, so the line numbers in a
//! driver's error log still point into src/shader.frag.

use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=src/shader.frag");
    let src = fs::read_to_string("src/shader.frag").expect("src/shader.frag");
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("shader.frag");
    fs::write(out, strip(&src)).unwrap();
}

/// Drops `//` and `/* */` comments (keeping the newlines inside them), then each line's
/// surrounding whitespace, and squeezes runs of spaces and tabs to one.
fn strip(src: &str) -> String {
    let mut code = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                while chars.peek().is_some_and(|&c| c != '\n') {
                    chars.next();
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if c == '\n' {
                        code.push('\n');
                    }
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            _ => code.push(c),
        }
    }
    code.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n") + "\n"
}
