use std::io::{self, Result};

use nu_protocol::{
    engine::{EngineState, Stack},
    PipelineData, Value,
};
use tui::layout::Rect;

use crate::{
    nu_common::{collect_pipeline, has_simple_value, is_ignored_command, run_nu_command},
    pager::Frame,
    views::{Layout, Orientation, Preview, RecordView, View, ViewConfig},
};

use super::{HelpExample, HelpManual, ViewCommand};

#[derive(Debug, Default, Clone)]
pub struct NuCmd {
    command: String,
}

impl NuCmd {
    pub fn new() -> Self {
        Self {
            command: String::new(),
        }
    }

    pub const NAME: &'static str = "nu";
}

impl ViewCommand for NuCmd {
    type View = NuView<'static>;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn usage(&self) -> &'static str {
        ""
    }

    fn help(&self) -> Option<HelpManual> {
        Some(HelpManual {
            name: "nu",
            description:
                "Run a Nushell command. The data currently being explored is piped into it.",
            arguments: vec![],
            input: vec![],

            examples: vec![
                HelpExample {
                    example: "where type == 'file'",
                    description: "Filter data to show only rows whose type is 'file'",
                },
                HelpExample {
                    example: "get scope.examples",
                    description: "Navigate to a deeper value inside the data",
                },
                HelpExample {
                    example: "open Cargo.toml",
                    description: "Open a Cargo.toml file",
                },
            ],
        })
    }

    fn parse(&mut self, args: &str) -> Result<()> {
        self.command = args.trim().to_owned();

        Ok(())
    }

    fn spawn(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        value: Option<Value>,
    ) -> Result<Self::View> {
        if is_ignored_command(&self.command) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "The command is ignored",
            ));
        }

        let value = value.unwrap_or_default();

        let pipeline = PipelineData::Value(value, None);
        let pipeline = run_nu_command(engine_state, stack, &self.command, pipeline)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if let PipelineData::Value(Value::Error { error }, ..) = pipeline {
            return Err(io::Error::new(io::ErrorKind::Other, error.to_string()));
        }

        let is_record = matches!(pipeline, PipelineData::Value(Value::Record { .. }, ..));

        let (columns, values) = collect_pipeline(pipeline);

        if has_simple_value(&values) {
            let config = &engine_state.config;
            let text = values[0][0].into_abbreviated_string(config);
            return Ok(NuView::Preview(Preview::new(&text)));
        }

        let mut view = RecordView::new(columns, values);

        if is_record {
            view.set_orientation_current(Orientation::Left);
        }

        Ok(NuView::Records(view))
    }
}

pub enum NuView<'a> {
    Records(RecordView<'a>),
    Preview(Preview),
}

impl View for NuView<'_> {
    fn draw(&mut self, f: &mut Frame, area: Rect, cfg: ViewConfig<'_>, layout: &mut Layout) {
        match self {
            NuView::Records(v) => v.draw(f, area, cfg, layout),
            NuView::Preview(v) => v.draw(f, area, cfg, layout),
        }
    }

    fn handle_input(
        &mut self,
        engine_state: &EngineState,
        stack: &mut Stack,
        layout: &Layout,
        info: &mut crate::pager::ViewInfo,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::pager::Transition> {
        match self {
            NuView::Records(v) => v.handle_input(engine_state, stack, layout, info, key),
            NuView::Preview(v) => v.handle_input(engine_state, stack, layout, info, key),
        }
    }

    fn show_data(&mut self, i: usize) -> bool {
        match self {
            NuView::Records(v) => v.show_data(i),
            NuView::Preview(v) => v.show_data(i),
        }
    }

    fn collect_data(&self) -> Vec<crate::nu_common::NuText> {
        match self {
            NuView::Records(v) => v.collect_data(),
            NuView::Preview(v) => v.collect_data(),
        }
    }

    fn exit(&mut self) -> Option<Value> {
        match self {
            NuView::Records(v) => v.exit(),
            NuView::Preview(v) => v.exit(),
        }
    }

    fn setup(&mut self, config: ViewConfig<'_>) {
        match self {
            NuView::Records(v) => v.setup(config),
            NuView::Preview(v) => v.setup(config),
        }
    }
}
