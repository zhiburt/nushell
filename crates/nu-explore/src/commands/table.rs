use std::io::Result;

use nu_protocol::{
    engine::{EngineState, Stack},
    Value,
};

use crate::{
    nu_common::collect_input,
    views::{Orientation, RecordView},
};

use super::{HelpManual, Shortcode, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct TableCmd {
    // todo: add arguments to override config right from CMD
}

impl TableCmd {
    pub fn new() -> Self {
        Self {}
    }

    pub const NAME: &'static str = "table";
}

impl ViewCommand for TableCmd {
    type View = RecordView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        #[rustfmt::skip]
        let shortcuts = vec![
            Shortcode::new("Up",     "",        "Moves the cursor or viewport one row up"),
            Shortcode::new("Down",   "",        "Moves the cursor or viewport one row down"),
            Shortcode::new("Left",   "",        "Moves the cursor or viewport one column left"),
            Shortcode::new("Right",  "",        "Moves the cursor or viewport one column right"),
            Shortcode::new("PgDown", "view",    "Moves the cursor or viewport one page of rows down"),
            Shortcode::new("PgUp",   "view",    "Moves the cursor or viewport one page of rows up"),
            Shortcode::new("Esc",    "",        "Exits cursor mode. Exits the just explored dataset."),
            Shortcode::new("i",      "view",    "Enters cursor mode to inspect individual cells"),
            Shortcode::new("t",      "view",    "Transpose table, so that columns become rows and vice versa"),
            Shortcode::new("Enter",  "cursor",  "In cursor mode, explore the data of the selected cell"),
        ];

        Some(HelpManual {
            name: "table",
            description: "Display a table view",
            arguments: vec![],
            examples: vec![],
            input: shortcuts,
        })
    }

    fn parse(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn spawn(
        &mut self,
        _: &EngineState,
        _: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        let value = value.unwrap_or_default();
        let is_record = matches!(value, Value::Record { .. });

        let (columns, data) = collect_input(value);

        let mut view = RecordView::new(columns, data);

        if is_record {
            view.set_orientation_current(Orientation::Left);
        }

        Ok(view)
    }
}
