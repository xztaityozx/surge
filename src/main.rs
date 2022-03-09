use crate::io::Write;
use regex::Regex;
use std::thread;
use std::{
    env,
    process::{Command, Stdio},
};
use std::{io, io::BufRead};

const BUF_SIZE: usize = 1024;

pub fn log_fatal(msg: &str) -> ! {
    eprint!("{}: {}", env!("CARGO_PKG_NAME"), msg);
    std::process::exit(1);
}

fn main() {
    let stdin = io::stdin();
    let cmd = &["jq", "-s", "add"];
    let (send, recv) = multiqueue::broadcast_queue(10);
    let mut handles = vec![];

    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    drop(send);
                    break;
                }

                let sender = send.clone();

                handles.push(thread::spawn(move|| {
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
                    return output.stdout.clone();
                }))
            }
            Err(e) => {
                eprint!("{}", e);
                return;
            }
        }
    }

    recv.unsubscribe();

    let mut stdout = io::stdout();
    for t in handles {
        let buf = t.join().unwrap_or_else(|_| log_fatal("failed to wait complete sub process"));
        stdout.write_all(&buf).unwrap_or_else(|e| log_fatal(&e.to_string()));
    }
}
