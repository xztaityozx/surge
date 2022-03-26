#[cfg(test)]
mod tests {
    use std::{sync::Arc, thread};

    use crate::output_stream::{spawn, OutputStreamOption};

    #[test]
    fn it_spawn_ok_on_success() {
        let (tx, rx) = crossbeam::channel::bounded(10);
        let handle = spawn(
            rx,
            Arc::new(OutputStreamOption {
                output_delimiter: " ".to_string(),
                suppress_fail: false,
                log_fatal,
            }),
        );

        tx.send(thread::spawn(|| {
            sub_process::sub_process::SubProcessResult {
                success: true,
                input: "input string".as_bytes().to_vec(),
                output: "output string".as_bytes().to_vec(),
                cmd: Arc::new(Vec::new()),
            }
        }))
        .unwrap();

        drop(tx);
        assert!(handle.join().is_ok());
    }

    #[test]
    fn it_spawn_ok_on_failed() {
        let (tx, rx) = crossbeam::channel::bounded(10);
        let handle = spawn(
            rx,
            Arc::new(OutputStreamOption {
                output_delimiter: " ".to_string(),
                suppress_fail: true,
                log_fatal,
            }),
        );

        tx.send(thread::spawn(|| {
            sub_process::sub_process::SubProcessResult {
                success: false,
                input: "input string".as_bytes().to_vec(),
                output: "output string".as_bytes().to_vec(),
                cmd: Arc::new(Vec::new()),
            }
        }))
        .unwrap();

        drop(tx);
        assert!(handle.join().is_ok());
    }

    fn log_fatal(_s: &str) -> ! {
        std::process::exit(1)
    }
}

pub mod output_stream {
    use std::{
        io::{BufRead, BufReader, BufWriter, Write},
        sync::Arc,
        thread::{self, JoinHandle},
    };

    use crossbeam::channel::Receiver;
    use sub_process::sub_process::SubProcessHandle;

    /// options for output stream
    pub struct OutputStreamOption {
        /// each line of output joined with `output_delimiter`
        pub output_delimiter: String,
        /// suppress command failed
        pub suppress_fail: bool,
        /// specify logger for fatal
        pub log_fatal: fn(&str) -> !,
    }

    /// spawn output sub process
    pub fn spawn(
        rx: Receiver<SubProcessHandle>,
        option: Arc<OutputStreamOption>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let stdout = std::io::stdout();
            let mut writer = BufWriter::new(stdout.lock());

            for handle in rx {
                let result = handle
                    .join()
                    .unwrap_or_else(|_| (option.log_fatal)("failed to spawn sub process"));
                if result.success {
                    // on normal exit
                    let mut lines = BufReader::new(result.output.as_slice())
                        .lines()
                        .map(|l| l.unwrap_or_else(|e| (option.log_fatal)(&e.to_string())));

                    // join with output_delimiter
                    if let Some(line) = lines.next() {
                        write!(writer, "{}", &line)
                            .unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));

                        for line in lines {
                            write!(writer, "{}", option.output_delimiter)
                                .unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
                            write!(writer, "{}", &line)
                                .unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
                        }
                    } else {
                        writeln!(writer).unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
                    }
                } else if !option.suppress_fail {
                    (option.log_fatal)(&result.error_msg())
                }

                writeln!(writer).unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
            }
        })
    }
}
