use actix::{Actor, Address, Context};
use failure::Error;

pub struct Interpreter { }

impl Interpreter {
    pub fn launch() -> Result<Address<Self>, Error> {
        Ok(Self { }.start())
    }
}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
