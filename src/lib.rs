#![allow(dead_code)]

use actix::prelude::*;
use crossbeam_channel::Sender;
use futures::FutureExt;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{
    thread::JoinHandle,
    time::{Duration, SystemTime},
};

struct Job<I, O> {
    input: I,
    output: futures::channel::oneshot::Sender<MyResult<O>>,
}
struct MyActor {
    worker: JoinHandle<()>,
    queue: Sender<Job<usize, usize>>,
    generation: Arc<AtomicUsize>,
}

const MAILBOX_SIZE: usize = 16;
impl MyActor {
    fn new() -> MyActor {
        let (tx, rx) = crossbeam_channel::bounded::<Job<usize, usize>>(MAILBOX_SIZE);
        let generation = Arc::new(AtomicUsize::new(0));
        let signal = generation.clone();
        MyActor {
            worker: std::thread::spawn(move || {
                while let Ok(Job { input, output }) = rx.recv() {
                    let init = signal.load(Ordering::Relaxed);
                    println!("[{:?}] {} starting", SystemTime::now(), input);
                    for _ in 0..input {
                        println!("[{:?}] {} sleeping...", SystemTime::now(), input);
                        if signal.load(Ordering::Relaxed) != init {
                            println!("[{:?}] {} interrupted!", SystemTime::now(), input);
                            let _ = output.send(Err(MyErr::Interrupted));
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    println!("[{:?}] {} done", SystemTime::now(), input);
                    let _ = output.send(Ok(input));
                }
            }),
            queue: tx,
            generation,
        }
    }
}

impl Actor for MyActor {
    type Context = Context<Self>;
}

#[derive(Debug)]
enum Command {
    Echo(usize),
    Interrupt,
}
type MyResult<T> = Result<T, MyErr>;
#[derive(Debug, Clone, Eq, PartialEq)]
enum MyErr {
    Interrupted,
    Busy,
}
impl Message for Command {
    type Result = MyResult<usize>;
}

impl Handler<Command> for MyActor {
    type Result = ResponseFuture<MyResult<usize>>;

    fn handle(&mut self, msg: Command, _ctx: &mut Self::Context) -> Self::Result {
        println!("[{:?}] recv {:?}", SystemTime::now(), msg);
        match msg {
            Command::Interrupt => {
                let g = self.generation.fetch_add(1, Ordering::Relaxed);
                Box::pin(std::future::ready(Ok(g)))
            }
            Command::Echo(n) => {
                let (tx, rx) = futures::channel::oneshot::channel();
                let job = Job {
                    input: n,
                    output: tx,
                };
                match self.queue.try_send(job) {
                    Ok(_) => Box::pin(rx.map(|r| r.unwrap())),
                    Err(_) => Box::pin(std::future::ready(Err(MyErr::Busy))),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Command, MyActor, MyErr, MAILBOX_SIZE};
    use actix::prelude::*;
    use std::time::Duration;

    #[test]
    fn hello() {
        assert_eq!(2 + 2, 4);
    }

    #[actix_rt::test]
    async fn interrupt_test() {
        let my_actor = MyActor::new().start();

        let m1 = my_actor.send(Command::Echo(1_000));
        actix_rt::time::delay_for(Duration::from_millis(100)).await;
        let m2 = my_actor.send(Command::Interrupt);
        assert_eq!(m1.await.unwrap(), Err(MyErr::Interrupted));
        assert_eq!(m2.await.unwrap(), Ok(0));
    }

    #[actix_rt::test]
    async fn busy_test() {
        let my_actor = MyActor::new().start();

        for _ in 0..MAILBOX_SIZE {
            my_actor.try_send(Command::Echo(1_000)).unwrap();
        }
        let m = my_actor.send(Command::Echo(1_000));
        assert_eq!(m.await.unwrap(), Err(MyErr::Busy));
    }
}
