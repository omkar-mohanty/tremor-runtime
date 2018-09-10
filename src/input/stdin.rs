use input::{Input as InputT, INPUT_ERR, INPUT_OK};
use pipeline::Msg;
#[cfg(feature = "try_spmc")]
use spmc;
use std::io::{self, BufRead, BufReader};
use std::sync::mpsc;

pub struct Input {
    ff: bool,
}

impl Input {
    pub fn new(opts: &str) -> Self {
        if opts == "fire-and-forget" {
            Self { ff: true }
        } else {
            Self { ff: false }
        }
    }
}

impl InputT for Input {
    fn enter_loop(&mut self, pipelines: Vec<mpsc::SyncSender<Msg>>) {
        let stdin = io::stdin();
        let stdin = BufReader::new(stdin);
        for line in stdin.lines() {
            match line {
                Ok(line) => {
                    INPUT_OK.inc();
                    let msg = Msg::new(None, line);
                    if self.ff {
                        pipelines[0].try_send(msg).unwrap();
                    } else {
                        pipelines[0].send(msg).unwrap();
                    }
                }
                Err(_) => INPUT_ERR.inc(),
            }
        }
    }

    #[cfg(feature = "try_spmc")]
    fn enter_loop2(&mut self, pipelines: Vec<spmc::Sender<Msg>>) {
        let stdin = io::stdin();
        let stdin = BufReader::new(stdin);
        for line in stdin.lines() {
            debug!("Line: {:?}", line);
            match line {
                Ok(line) => {
                    INPUT_OK.inc();
                    let msg = Msg::new(None, line);
                    let _ = pipelines[0].send(msg);
                }
                Err(_) => INPUT_ERR.inc(),
            }
        }
    }
}
