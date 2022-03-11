use crate::io::Write;
use regex::Regex;
use std::process::ExitStatus;
use std::thread;
use std::thread::JoinHandle;
use std::process::{Command, Stdio};
use std::{io, io::BufRead};
use crossbeam::channel::{Sender, Receiver};
use clap::Parser;

const BUF_SIZE: usize = 1024;

#[macro_use]
extern crate log;

pub fn log_fatal(msg: &str) -> ! {
    error!("{}", msg);
    std::process::exit(1);
}


pub struct SubProcessResult {
    exit_code: ExitStatus,
    output: Vec<u8>,
    cmd: Vec<String>,
    input: String
}
type SubProcessHandle = JoinHandle<SubProcessResult>;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Arg {
    /// command
    command: String,

    /// arguments for command
    arguments: Vec<String>,

    /// delimiter for input
    #[clap(short = 'd', long, default_value_t = String::from(" "))]
    input_delimiter: String,

    /// delimiter for output
    #[clap(short = 'D', long, default_value_t = String::from(" "))]
    output_delimiter: String,

    #[clap(short = 'g', long)]
    use_regex: bool,
}

struct Replacer {
    delm: String,
    regex: Option<regex::Regex>
}

impl Replacer {
    fn replace_all(self,line: &[u8]) -> Result<String, regex::Error> {
        let l = String::from_utf8_lossy(line);
        match self.regex {
            None => Ok(l.replace(&self.delm, "\n")),
            Some(r) => Ok(r.replace_all(&l, "\n").to_string())
        }
    }
}

fn main() {
    let arg = Arg::parse();

    env_logger::init();
    let stdin = io::stdin();
    let cmd = [[arg.command].to_vec(), arg.arguments].concat();
    let (tx, rx):(Sender<SubProcessHandle>, Receiver<SubProcessHandle>) = crossbeam::channel::bounded(10);
    let replacer = Replacer {
        delm: arg.input_delimiter.clone(),
        regex: if arg.use_regex {
            Some(Regex::new(&arg.input_delimiter).unwrap_or_else(|e| log_fatal(&e.to_string())))
        } else {
            None
        }
    };

    let output_handle = thread::spawn(|| {
        let mut stdout = io::stdout();
        for handle in rx {
            let result = handle.join().unwrap();
            if result.exit_code.success() {
                stdout.write_all(&result.output).unwrap();
            }else{
                log_fatal(&("sub process exit code is not 0\ninput:\n\t".to_owned() + &result.input + "\ncommand:\n\t" + &result.cmd.join(" ")  + "\nerror:\n\t" + &String::from_utf8_lossy(&result.output)))
            }
            stdout.write_all("\n".as_bytes()).unwrap();
        }
    });

    loop {
        let mut buf = Vec::with_capacity(BUF_SIZE);
        match stdin.lock().read_until(b'\n', &mut buf) {
            Ok(n) => {
                if n == 0 {
                    drop(tx);
                    break;
                }

                let sender = tx.clone();
                let cmd = cmd.clone();
                let output_delimiter = arg.output_delimiter.clone();

                sender.send(thread::spawn(move|| {
                    let lines = replacer.replace_all(&buf).unwrap_or_else(|e| log_fatal(&e.to_string()));
                    let mut child = Command::new(&cmd[0])
                        .args(&cmd[1..])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
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

                    let buf = if output.status.success()  { 
                        String::from_utf8_lossy(&output.stdout).replace('\n',&output_delimiter).trim_end().as_bytes().to_vec()
                    }  else { output.stderr };
                    SubProcessResult { exit_code: output.status, output: buf, input: lines.trim().to_string(), cmd: cmd.to_vec() }
                })).unwrap_or_else(|e| log_fatal(&e.to_string()));
            }
            Err(e) => {
                eprint!("{}", e);
                return;
            }
        }
    }

    output_handle.join().unwrap_or_else(|_| log_fatal(""));
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

}
