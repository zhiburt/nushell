use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::viewers::scroll::pager::{Pager, Transition};

use super::{HelpManual, SimpleCommand};

#[derive(Default)]
pub struct QuitCmd;

impl QuitCmd {
    pub const NAME: &'static str = "q";
}

impl SimpleCommand for QuitCmd {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "q",
            description: "Quite a programm",
            arguments: vec![],
            examples: vec![],
        })
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn react(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        _: &mut Pager<'_>,
        _: Option<Value>,
    ) -> Result<Transition> {
        Ok(Transition::Exit)
    }
}
