use nu_engine::column::get_columns;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError, Signature,
    Span, Value,
};

#[derive(Clone)]
pub struct Length;

impl Command for Length {
    fn name(&self) -> &str {
        "length"
    }

    fn usage(&self) -> &str {
        "Count the number of elements in the input."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("length")
            .switch("column", "Show the number of columns in a table", Some('c'))
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let col = call.has_flag("column");
        if col {
            length_col(engine_state, call, input)
        } else {
            length_row(call, input)
        }
    }
}

// this simulates calling input | columns | length
fn length_col(
    engine_state: &EngineState,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    length_row(
        call,
        getcol(engine_state, call.head, input)
            .expect("getcol() should not fail used in column command"),
    )
}

fn length_row(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    match input {
        PipelineData::Value(Value::Nothing { .. }, ..) => Ok(Value::Int {
            val: 0,
            span: call.head,
        }
        .into_pipeline_data()),
        _ => Ok(Value::Int {
            val: input.into_iter().count() as i64,
            span: call.head,
        }
        .into_pipeline_data()),
    }
}

fn getcol(
    engine_state: &EngineState,
    span: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    match input {
        PipelineData::Value(
            Value::List {
                vals: input_vals,
                span,
            },
            ..,
        ) => {
            let input_cols = get_columns(&input_vals);
            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::ListStream(stream, ..) => {
            let v: Vec<_> = stream.into_iter().collect();
            let input_cols = get_columns(&v);

            Ok(input_cols
                .into_iter()
                .map(move |x| Value::String { val: x, span })
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(..) | PipelineData::StringStream(..) | PipelineData::ByteStream(..) => {
            let cols = vec![];
            let vals = vec![];
            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
    }
}
