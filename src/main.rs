use crate::io::Write;
use regex::Regex;
use std::{
    env,
    process::{Command, Stdio}
};
use std::{io, io::BufRead};

const BUF_SIZE: usize = 1024;

pub fn log_fatal(msg: &str) -> ! {
    eprint!("{}: {}", env!("CARGO_PKG_NAME"), msg);
    std::process::exit(1);
}

pub enum ExecError {
    StdinOpenFailed,
    StdoutOpenFailed,
    Io(std::io::Error),
}

fn main() {
    let delimiter = Regex::new("\\s+").unwrap_or_else(|e| log_fatal(&e.to_string()));
    let stdin = io::stdin();
    let cmds = &["jq", "-s", "add"];
    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }

                let mut child = Command::new(cmds[0])
                    .args(&cmds[1..])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap_or_else(|e| log_fatal(&e.to_string()));

                let stdin = child.stdin.as_mut().unwrap_or_else(|| log_fatal("failed to open stdin"));

                let line = String::from_utf8_lossy(&buf).to_string();
                let mut left = 0;
                for m in delimiter.find_iter(&line) {
                    let right = m.start();
                    let field = line[left..right].to_string() + "\n";
                    stdin.write_all(field.as_bytes()).unwrap_or_else(|e| log_fatal(&e.to_string()));
                    left = m.end();
                }
                stdin.flush().unwrap_or_else(|e| log_fatal(&e.to_string()));

                let output = child.wait_with_output().unwrap_or_else(|e| log_fatal(&e.to_string()));
                println!("{}", String::from_utf8_lossy(&output.stdout).replace('\n', " "));
            }
            Err(e) => {
                eprint!("{e}");
                return;
            }
        }
    }
}
