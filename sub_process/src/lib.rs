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
    use std::{sync::Arc, thread::JoinHandle};

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
}
