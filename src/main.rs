use crate::io::Write;
use anstyle;
use clap::{CommandFactory, Parser};
use clap_complete::generate;
use crossbeam::channel::{Receiver, Sender};
use env_logger::Builder;
use regex::Regex;
use std::sync::mpsc::SendError;
use std::sync::Arc;
use std::{io, io::BufRead};
mod output;
use crate::output::stream::{spawn, OutputStreamOption};
mod sub_process;
use crate::sub_process::sub_process::{SubProcess, SubProcessHandle};

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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
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
    suppress_fail: bool,

    /// maximum number of parallels
    #[clap(short = 'P', long, default_value_t = 1)]
    number_of_parallel: usize,

    /// generate completion script (bash,zsh,fish,powershell)
    #[clap(long, value_name = "SHELL")]
    completion: Option<clap_complete::Shell>,
}

// regex_process は行を正規表現で分割して cmd の stdin に流し込む
fn regex_process(
    delm: &regex::Regex,
    cmd: Vec<String>,
    tx: &Sender<SubProcessHandle>,
) -> Result<(), SendError<SubProcessHandle>> {
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
                    tx: tx.clone(),
                    log_fatal,
                };

                sub_process
                    .spawn(
                        delm.replace_all(&String::from_utf8_lossy(&buf), "\n")
                            .to_string()
                            .as_bytes()
                            .to_vec(),
                    )
                    .unwrap_or_else(|e| log_fatal(&e.to_string()));
            }
            Err(e) => log_fatal(&e.to_string()),
        }
    }

    Ok(())
}

// string_process は行を文字列で分割して cmd の stdin に流し込む
fn string_process(
    delm: String,
    cmd: Vec<String>,
    tx: &Sender<SubProcessHandle>,
) -> Result<(), SendError<SubProcessHandle>> {
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
                    tx: tx.clone(),
                    log_fatal,
                };

                let line = String::from_utf8_lossy(&buf);

                sub_process
                    .spawn(line.replace(&delm, "\n").to_string().as_bytes().to_vec())
                    .unwrap_or_else(|e| log_fatal(&e.to_string()));
            }
            Err(e) => log_fatal(&e.to_string()),
        }
    }

    Ok(())
}

fn main() {
    let arg = Args::parse();

    if let Some(shell) = arg.completion {
        generate(shell, &mut Args::command(), APP_NAME, &mut io::stdout());
        std::process::exit(0);
    }

    Builder::new()
        .format(|buf, record| -> Result<(), io::Error> {
            writeln!(
                buf,
                "[{}{}{} {}] {}",
                anstyle::Style::new()
                    .fg_color(Some(anstyle::AnsiColor::Red.into()))
                    .render(),
                record.level().as_str(),
                anstyle::Reset::render(anstyle::Reset),
                APP_NAME,
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();

    let cmd = [[arg.command].to_vec(), arg.arguments].concat();
    let (tx, rx): (Sender<SubProcessHandle>, Receiver<SubProcessHandle>) =
        crossbeam::channel::bounded(arg.number_of_parallel);

    let output_handle = spawn(
        rx,
        Arc::new(OutputStreamOption {
            output_delimiter: arg.output_delimiter,
            log_fatal,
            suppress_fail: arg.suppress_fail,
        }),
    );

    match arg.regex {
        Some(r) => {
            regex_process(&r, cmd, &tx).unwrap_or_else(|e| log_fatal(&e.to_string()));
        }
        None => {
            string_process(arg.input_delimiter, cmd, &tx)
                .unwrap_or_else(|e| log_fatal(&e.to_string()));
        }
    }
    drop(tx);

    output_handle
        .join()
        .unwrap_or_else(|_| log_fatal("failed to spawn output thread"));
}
