use actix::prelude::*;

struct MyActor;

impl Actor for MyActor {
    type Context = Context<Self>;
}

enum Command {
    Echo(usize),
}
impl Message for Command {
    type Result = Result<usize, String>;
}

impl Handler<Command> for MyActor {
    type Result = Result<usize, String>;

    fn handle(&mut self, msg: Command, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Command::Echo(n) => Ok(n),
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
        assert_eq!(my_actor.send(Command::Echo(42)).await.unwrap(), Ok(42));
    }
}
