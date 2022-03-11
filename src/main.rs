use crate::io::Write;
use chrono::Local;
use env_logger::Builder;
use regex::Regex;
use std::sync::Arc;
use std::{io, io::BufRead};
use std::sync::mpsc::SendError;
use crossbeam::channel::{Sender, Receiver};
use clap::Parser;
use ansi_term::Color::Red;
use std::thread::JoinHandle;
use std::process::{Command, Stdio};
use std::process::ExitStatus;
use std::thread;

static INPUT_DELIMITER_GROUP_NAME: &str = "INPUT_DELIMITER_GROUP";
const BUF_SIZE: usize = 1024;
static APP_NAME: &str = env!("CARGO_PKG_NAME");

#[macro_use]
extern crate log;

// print error log and exit
pub fn log_fatal(msg: &str) -> ! {
    error!("{}", msg);
    std::process::exit(1);
}

pub struct SubProcessResult {
    exit_code: ExitStatus,
    input: Vec<u8>,
    output: Vec<u8>,
    cmd: Arc<Vec<String>>,
}

impl SubProcessResult {
    fn error_msg(self) -> String {
        [
            "sub process exit code is not 0".to_string(),
            "input:".to_string(),
            format!("{}", String::from_utf8_lossy(&self.input)),
            "".to_owned(),
            "command:".to_string(),
            format!("\t{}", self.cmd.join(" ")),
            "".to_owned(),
            "output:".to_string(),
            format!("\t{}", String::from_utf8_lossy(&self.output))
        ].join("\n")
    }
}

type SubProcessHandle = JoinHandle<SubProcessResult>;

pub struct SubProcess {
    cmd: Arc<Vec<String>>,
    tx: Sender<SubProcessHandle>
}

impl SubProcess {
    fn spawn(self, input_buf: Vec<u8>) -> Result<(), SendError<SubProcessHandle>> {
        let handle = thread::spawn(move ||{
            let mut child = Command::new(&self.cmd[0])
                .args(&self.cmd[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap_or_else(|e| {
                    log_fatal(&[
                        "failed to spawn sub process".to_owned(),
                        (e.to_string())
                    ].join(": "))
                });

            let stdin = child
                .stdin
                .as_mut()
                .unwrap_or_else(|| log_fatal("failed to open sub process stdin"));

            stdin.write_all(&input_buf).unwrap_or_else(|e| log_fatal(&e.to_string()));
            stdin.flush().unwrap_or_else(|e| log_fatal(&e.to_string()));

            let output = child
                .wait_with_output()
                .unwrap_or_else(|e| {
                    log_fatal(&[
                        "failed to wait sub process stdout".to_owned(),
                        (e.to_string())
                    ].join(": "))
                });

            let output_buf = if output.status.success()  { output.stdout }  else { output.stderr };
            SubProcessResult { 
                exit_code: output.status, 
                input: input_buf.to_vec(),
                output: output_buf, 
                cmd: self.cmd 
            }
        });

        self.tx.send(handle).unwrap();
        Ok(())
    }
}


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Arg {
    /// command
    command: String,

    /// arguments for command
    arguments: Vec<String>,

    /// delimiter for input
    #[clap(short = 'd', long, default_value_t = String::from(" "), group = INPUT_DELIMITER_GROUP_NAME)]
    input_delimiter: String,

    /// delimiter for output
    #[clap(short = 'D', long, default_value_t = String::from(" "))]
    output_delimiter: String,

    /// split by regex
    #[clap(short = 'g', long, group = INPUT_DELIMITER_GROUP_NAME)]
    regex: Option<Regex>,

    /// continue other process even if one of the sub process fails
    #[clap(long)]
    suppress_fail: bool
}

// regex_process は行を正規表現で分割して cmd の stdin に流し込む
fn regex_process(delm: &regex::Regex, cmd: Vec<String>, tx: &Sender<SubProcessHandle>) -> Result<(), SendError<SubProcessHandle>> {
    let command_line = Arc::new(cmd);

    let stdin = io::stdin();

    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }

                let sub_process = SubProcess {
                    cmd: Arc::clone(&command_line),
                    tx: tx.clone()
                };

                sub_process.spawn(
                    delm.replace_all(&String::from_utf8_lossy(&buf), "\n")
                        .to_string()
                        .as_bytes()
                        .to_vec()
                )
                    .unwrap();
            },
            Err(e) => {
                log_fatal(&e.to_string())
            }
        }
    }

    Ok(())
}

// string_process は行を文字列で分割して cmd の stdin に流し込む
fn string_process(delm: String, cmd: Vec<String>, tx: &Sender<SubProcessHandle>) -> Result<(), SendError<SubProcessHandle>> {
    let command_line = Arc::new(cmd);
    let stdin = io::stdin();

    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    break;
                }

                let sub_process = SubProcess {
                    cmd: Arc::clone(&command_line),
                    tx: tx.clone()
                };

                let line = String::from_utf8_lossy(&buf);

                sub_process.spawn(
                    line.replace(&delm, "\n")
                        .to_string()
                        .as_bytes()
                        .to_vec()
                )
                    .unwrap();
            },
            Err(e) => {
                log_fatal(&e.to_string())
            }
        }
    }

    Ok(())
}


fn main() {
    let arg = Arg::parse();

    Builder::new()
        .format(|buf, record| -> Result<(), io::Error> {
            writeln!(
                buf, 
                "[{} {} {}] {}",
                Local::now().format("%F %T"),
                Red.paint(record.level().to_string()),
                APP_NAME,
                record.args(),
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    let cmd = [[arg.command].to_vec(), arg.arguments].concat();
    let (tx, rx):(Sender<SubProcessHandle>, Receiver<SubProcessHandle>) = crossbeam::channel::bounded(10);
    let suppress_fail = arg.suppress_fail;

    match arg.regex {
        Some(r) => {
            regex_process(&r,cmd,&tx).unwrap_or_else(|e| log_fatal(&e.to_string()));
        },
        None => {
            string_process(arg.input_delimiter, cmd, &tx).unwrap_or_else(|e| log_fatal(&e.to_string()));
        }
    }
    drop(tx);

    let mut stdout = io::stdout();
    for handle in rx {
        match handle.join() {
            Ok(result) => {
                if result.exit_code.success() {
                    stdout.write_all(
                        String::from_utf8_lossy(&result.output).trim_end().as_bytes()
                    ).unwrap_or_else(|e| log_fatal(&e.to_string()));
                } else if !suppress_fail {
                    log_fatal(&result.error_msg())
                }
                stdout.write_all("\n".as_bytes()).unwrap_or_else(|e| log_fatal(&e.to_string()));
            },
            Err(_) => {
                log_fatal("failed to spawn sub process");
            }
        }
    }
}
