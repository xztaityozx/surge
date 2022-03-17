#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
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
