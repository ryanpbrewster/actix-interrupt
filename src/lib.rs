#![allow(dead_code)]

use actix::prelude::*;
use futures::FutureExt;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use threadpool::ThreadPool;

struct MyActor {
    pool: ThreadPool,
    generation: Arc<AtomicUsize>,
}
impl MyActor {
    fn new() -> MyActor {
        MyActor {
            pool: ThreadPool::new(1),
            generation: Arc::new(AtomicUsize::new(0)),
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
                let signal = self.generation.clone();
                let init = signal.load(Ordering::Relaxed);
                self.pool.execute(move || {
                    println!("[{:?}] {} starting", SystemTime::now(), n);
                    for _ in 0..n {
                        println!("[{:?}] {} sleeping...!", SystemTime::now(), n);
                        if signal.load(Ordering::Relaxed) != init {
                            println!("[{:?}] {} interrupted!", SystemTime::now(), n);
                            let _ = tx.send(Err(MyErr::Interrupted));
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    println!("[{:?}] {} done", SystemTime::now(), n);
                    let _ = tx.send(Ok(n));
                });
                Box::pin(rx.map(|r| r.unwrap()))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Command, MyActor, MyErr};
    use actix::prelude::*;
    use std::time::Duration;

    #[test]
    fn hello() {
        assert_eq!(2 + 2, 4);
    }

    #[actix_rt::test]
    async fn my_test() {
        let my_actor = MyActor::new().start();

        let m1 = my_actor.send(Command::Echo(1_000));
        actix_rt::time::delay_for(Duration::from_millis(100)).await;
        let m2 = my_actor.send(Command::Interrupt);
        assert_eq!(m1.await.unwrap(), Err(MyErr::Interrupted));
        assert_eq!(m2.await.unwrap(), Ok(0));
    }
}
