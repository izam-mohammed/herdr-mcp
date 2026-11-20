//! A fake herdr: replays `tests/fixtures/herdr.jsonl` and records requests. A fixture with
//! `params` matches only a request with exactly those params.
use super::{Seen, bind};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct FakeHerdr {
    pub socket: PathBuf,
    pub seen: Seen,
}

impl FakeHerdr {
    /// Answer from the fixtures; `hang` accepts and never replies.
    pub fn start(tag: &str, hang: bool) -> FakeHerdr {
        let socket = super::socket_path(tag);
        let listener = bind(&socket);
        let seen: Seen = Seen::default();
        let fixtures: Vec<Value> = include_str!("../fixtures/herdr.jsonl")
            .lines()
            .map(|l| serde_json::from_str(l).expect("fixture line"))
            .collect();
        let log = seen.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let (fixtures, log) = (fixtures.clone(), log.clone());
                tokio::spawn(async move {
                    let mut stream = BufReader::new(stream);
                    let mut line = String::new();
                    if stream.read_line(&mut line).await.unwrap_or(0) == 0 || hang {
                        if hang {
                            std::future::pending::<()>().await;
                        }
                        return;
                    }
                    let req: Value = serde_json::from_str(&line).expect("request json");
                    log.lock().expect("log").push(req.clone());
                    let fixture = fixtures.iter().find(|f| {
                        f["method"] == req["method"]
                            && f.get("params").is_none_or(|p| *p == req["params"])
                    });
                    let mut reply = json!({"id": req["id"]});
                    match fixture {
                        Some(f) if f.get("error").is_some() => reply["error"] = f["error"].clone(),
                        Some(f) => reply["result"] = f["result"].clone(),
                        None => {
                            reply["error"] =
                                json!({"code":"unsupported_method","message":"no such method"});
                        }
                    }
                    let _ = stream
                        .get_mut()
                        .write_all(format!("{reply}\n").as_bytes())
                        .await;
                });
            }
        });
        FakeHerdr { socket, seen }
    }

    pub fn requests(&self) -> Vec<Value> {
        self.seen.lock().expect("log").clone()
    }
}
