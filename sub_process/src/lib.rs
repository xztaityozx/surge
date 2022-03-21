#[cfg(test)]
mod tests {
    use std::{
        process::{Command, Stdio},
        sync::Arc,
        thread::{self, JoinHandle},
    };

    use crossbeam::channel::{Receiver, Sender};

    use crate::sub_process::{SubProcess, SubProcessResult};

    #[test]
    fn it_sub_process_result_error_msg() {
        let cmd = &["a".to_string(), "b".to_string(), "c".to_string()];
        let input = "this is input".as_bytes();
        let output = "this is output".as_bytes();

        let expected = [
            "sub process exit code is not 0".to_string(),
            "input:".to_string(),
            format!("{}", String::from_utf8_lossy(input)),
            "".to_owned(),
            "command:".to_string(),
            format!("\t{}", cmd.join(" ")),
            "".to_owned(),
            "output:".to_string(),
            format!("\t{}", String::from_utf8_lossy(output)),
        ]
        .join("\n");

        let sub_process_result = SubProcessResult {
            cmd: Arc::new(cmd.to_vec()),
            input: input.to_vec(),
            output: output.to_vec(),
            success: true,
        };

        assert_eq!(expected, sub_process_result.error_msg())
    }

    #[test]
    fn it_sub_process_spawn_ng() {
        let cmd = Arc::new(["cargo", "--typo"].map(|s| s.to_string()).to_vec());
        let (tx, rx): (
            Sender<JoinHandle<SubProcessResult>>,
            Receiver<JoinHandle<SubProcessResult>>,
        ) = crossbeam::channel::bounded(3);
        let number_of_sub_process = 10;

        let receiver = thread::spawn(move || {
            for h in rx {
                let r = h.join().unwrap();
                assert!(!r.success);
            }
        });

        for _ in 1..number_of_sub_process {
            let sp = SubProcess {
                cmd: Arc::clone(&cmd),
                tx: tx.clone(),
                log_fatal: |_s| std::process::exit(1),
            };

            sp.spawn("".as_bytes().to_vec()).unwrap();
        }

        drop(tx);

        receiver.join().unwrap();
    }

    #[test]
    fn it_sub_process_spawn_ok() {
        let cmd = Arc::new(["cargo", "--help"].map(|s| s.to_string()).to_vec());
        let (tx, rx): (
            Sender<JoinHandle<SubProcessResult>>,
            Receiver<JoinHandle<SubProcessResult>>,
        ) = crossbeam::channel::bounded(3);
        let number_of_sub_process = 10;

        let expected = String::from_utf8_lossy(
            &Command::new("cargo")
                .arg("--help")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap()
                .stdout,
        )
        .to_string();

        let receiver = thread::spawn(move || {
            for h in rx {
                let r = h.join().unwrap();
                assert!(r.success);
                let actual = String::from_utf8_lossy(&r.output).to_string();
                assert_eq!(expected, actual);
            }
        });

        for _ in 1..number_of_sub_process {
            let sp = SubProcess {
                cmd: Arc::clone(&cmd),
                tx: tx.clone(),
                log_fatal: |_s| std::process::exit(1),
            };

            sp.spawn("".as_bytes().to_vec()).unwrap();
        }

        drop(tx);

        receiver.join().unwrap();
    }
}

pub mod sub_process {
    use std::{
        io::Write,
        process::{Command, Stdio},
        sync::{mpsc::SendError, Arc},
        thread::{self, JoinHandle},
    };

    use crossbeam::channel::Sender;

    pub struct SubProcessResult {
        /// status of sub process
        pub success: bool,
        /// input **line**
        pub input: Vec<u8>,
        /// output
        pub output: Vec<u8>,
        /// command line
        pub cmd: Arc<Vec<String>>,
    }
    impl SubProcessResult {
        /// make error message
        pub fn error_msg(self) -> String {
            [
                "sub process exit code is not 0".to_string(),
                "input:".to_string(),
                format!("{}", String::from_utf8_lossy(&self.input)),
                "".to_owned(),
                "command:".to_string(),
                format!("\t{}", self.cmd.join(" ")),
                "".to_owned(),
                "output:".to_string(),
                format!("\t{}", String::from_utf8_lossy(&self.output)),
            ]
            .join("\n")
        }
    }

    pub type SubProcessHandle = JoinHandle<SubProcessResult>;

    pub struct SubProcess {
        /// command line
        pub cmd: Arc<Vec<String>>,
        /// sub process handle channel
        pub tx: Sender<SubProcessHandle>,
        /// log and exit closure on failed create spawn thread
        pub log_fatal: fn(s: &str) -> !,
    }

    impl SubProcess {
        /// spawn sub process for each input line
        ///
        pub fn spawn(self, input_buf: Vec<u8>) -> Result<(), SendError<SubProcessHandle>> {
            let handle = thread::spawn(move || {
                let mut child = Command::new(&self.cmd[0])
                    .args(&self.cmd[1..])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap_or_else(|e| {
                        (self.log_fatal)(
                            &["failed to spawn sub process".to_owned(), (e.to_string())].join(": "),
                        )
                    });

                let stdin = child
                    .stdin
                    .as_mut()
                    .unwrap_or_else(|| (self.log_fatal)("failed to open sub process stdin"));

                stdin
                    .write_all(&input_buf)
                    .unwrap_or_else(|e| (self.log_fatal)(&e.to_string()));
                stdin
                    .flush()
                    .unwrap_or_else(|e| (self.log_fatal)(&e.to_string()));

                let output = child.wait_with_output().unwrap_or_else(|e| {
                    (self.log_fatal)(
                        &[
                            "failed to wait sub process stdout".to_owned(),
                            (e.to_string()),
                        ]
                        .join(": "),
                    )
                });

                let output_buf = if output.status.success() {
                    output.stdout
                } else {
                    output.stderr
                };
                SubProcessResult {
                    success: output.status.success(),
                    input: input_buf.to_vec(),
                    output: output_buf,
                    cmd: self.cmd,
                }
            });

            self.tx
                .send(handle)
                .unwrap_or_else(|e| (self.log_fatal)(&e.to_string()));
            Ok(())
        }
    }
}
