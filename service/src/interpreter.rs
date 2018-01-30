use std::fs::File;
use std::io::Read;

use actix::{Actor, Address, Context};
use failure::{Error, ResultExt};
use rlua::{Function, Lua};

use error::ErrorKind;

pub struct Interpreter {
    pub lua: Lua,
}

impl Interpreter {
    fn bootstrap(&mut self, contents: &str) -> Result<(), Error> {
        let globals = self.lua.globals();

        let registration_function = self.lua.create_function(
            |_, (client_type, new_event_handler, receipt_handler): (String, Function, Function)| {
                info!("received register call from script: client_type=`{}`", client_type);
                Ok(())
        }).context(ErrorKind::CreateRegisterFunction)?;
        globals.set("register", registration_function).context(ErrorKind::InjectRegisterFunction)?;

        self.lua.eval::<()>(&contents, None).context(ErrorKind::EvaluateLuaScript)?;

        Ok(())
    }

    pub fn launch(script_path: &str) -> Result<Address<Self>, Error> {
        let mut file = File::open(script_path).context(ErrorKind::LuaScriptNotFound)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).context(ErrorKind::ReadLuaScript)?;

        let mut interpreter = Self {
            lua: Lua::new(),
        };
        interpreter.bootstrap(&contents);

        Ok(interpreter.start())
    }
}

impl Actor for Interpreter {
    type Context = Context<Self>;
}
