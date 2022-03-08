use crate::io::Write;
use regex::Regex;
use std::sync::mpsc;
use std::{
    env,
    process::{Command, Stdio},
    thread,
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
    let stdin = io::stdin();
    let cmd = &["jq", "-s", "add"];
    let (tx, rx) = mpsc::channel();

    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    drop(tx);
                    break;
                }

                let sender = mpsc::Sender::clone(&tx);

                thread::spawn(move || {
                    let delimiter =
                        Regex::new("\\s+").unwrap_or_else(|e| log_fatal(&e.to_string()));
                    let line = &String::from_utf8_lossy(&buf).to_string();
                    let lines = delimiter.replace(line, "\n");
                    let mut child = Command::new(cmd[0])
                        .args(&cmd[1..])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                        .unwrap_or_else(|e| log_fatal(&e.to_string()));

                    let stdin = child
                        .stdin
                        .as_mut()
                        .unwrap_or_else(|| log_fatal("failed to open stdin"));
                    stdin
                        .write_all(lines.as_bytes())
                        .unwrap_or_else(|e| log_fatal(&e.to_string()));
                    stdin.flush().unwrap_or_else(|e| log_fatal(&e.to_string()));

                    let output = child
                        .wait_with_output()
                        .unwrap_or_else(|e| log_fatal(&e.to_string()));

                    sender
                        .send(output.stdout)
                        .unwrap_or_else(|e| log_fatal(&e.to_string()));
                });
            }
            Err(e) => {
                eprint!("{}", e);
                return;
            }
        }
    }

    for received in rx {
        print!("{}", String::from_utf8_lossy(&received));
    }
}
