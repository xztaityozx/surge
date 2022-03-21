#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::sub_process::SubProcessResult;

    #[test]
    fn it_error_msg() {
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
        ].join("\n");

        let sub_process_result = SubProcessResult{
            cmd: Arc::new(cmd.to_vec()),
            input: input.to_vec(),
            output: output.to_vec(),
            success: true
        };

        assert_eq!(expected, sub_process_result.error_msg())
    }
}

pub mod sub_process {
    use std::{sync::{Arc, mpsc::SendError}, thread::{JoinHandle, self}, process::{Command, Stdio}, io::Write};

    use crossbeam::channel::Sender;

    pub struct SubProcessResult {
        pub success: bool,
        pub input: Vec<u8>,
        pub output: Vec<u8>,
        pub cmd: Arc<Vec<String>>,
    }
    impl SubProcessResult {
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
        pub log_fatal: fn(s: &str) -> !
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
                stdin.flush().unwrap_or_else(|e| (self.log_fatal)(&e.to_string()));

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
