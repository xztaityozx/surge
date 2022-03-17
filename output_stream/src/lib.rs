#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

pub mod output_stream {
    use std::{thread::{JoinHandle, self}, io::{self, Write}, sync::Arc};

    use crossbeam::channel::Receiver;
    use sub_process::sub_process::SubProcessHandle;

    pub struct OutputStreamOption {
        pub output_delimiter: String,
        pub suppress_fail: bool,
        pub log_fatal: fn(&str) -> !,
    }

    pub fn spawn(rx: Receiver<SubProcessHandle>, option: Arc<OutputStreamOption>) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut stdout = io::stdout();
            for handle in rx {
                let result = handle.join().unwrap_or_else(|_| (option.log_fatal)("failed to spawn sub process"));
                if result.exit_code.success() {
                    stdout.write_all(
                        // 末尾の改行を取り除いてからじゃないと
                        // 末尾に余計な output_delimiter がついちゃう
                        String::from_utf8_lossy(&result.output).trim_end().replace('\n', &option.output_delimiter).as_bytes()
                    ).unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
                } else if !option.suppress_fail {
                    (option.log_fatal)(&result.error_msg())
                }
                stdout.write_all("\n".as_bytes()).unwrap_or_else(|e| (option.log_fatal)(&e.to_string()));
            }
        })
    }
}
