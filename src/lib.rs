use actix::prelude::*;
use std::time::{Instant, Duration};

struct MyActor;

impl Actor for MyActor {
    type Context = Context<Self>;
}

#[derive(Debug)]
enum Command {
    Echo(usize),
}
impl Message for Command {
    type Result = Result<usize, String>;
}

impl Handler<Command> for MyActor {
    type Result = ResponseActFuture<Self, Result<usize, String>>;

    fn handle(&mut self, msg: Command, _ctx: &mut Self::Context) -> Self::Result {
        println!("recv {:?}", msg);
        match msg {
            Command::Echo(n) => {
                let fut = async move {
                    println!("[{:?}] {} sleeping for 1s...", Instant::now(), n);
                    actix_rt::time::delay_for(Duration::from_millis(1_000)).await;
                    println!("[{:?}] {} waking up!", Instant::now(), n);
                    n * n
                };
                let defer = actix::fut::wrap_future(fut).map(|result, _actor, _ctx| {
                    Ok(result)
                });
                Box::pin(defer)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Command, MyActor};
    use actix::prelude::*;

    #[test]
    fn hello() {
        assert_eq!(2 + 2, 4);
    }

    #[actix_rt::test]
    async fn my_test() {
        let my_actor = MyActor.start();

        let m1 = my_actor.send(Command::Echo(42));
        let m2 = my_actor.send(Command::Echo(84));

        assert_eq!(m1.await.unwrap(), Ok(42));
        assert_eq!(m2.await.unwrap(), Ok(42));
    }
}
