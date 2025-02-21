use nu_engine::eval_block;
use nu_parser::{flatten_expression, parse};
use nu_protocol::{
    ast::Statement,
    engine::{EngineState, Stack, StateWorkingSet},
    PipelineData, Span,
};
use reedline::Completer;

const SEP: char = std::path::MAIN_SEPARATOR;

#[derive(Clone)]
pub struct NuCompleter {
    engine_state: EngineState,
}

impl NuCompleter {
    pub fn new(engine_state: EngineState) -> Self {
        Self { engine_state }
    }

    fn external_command_completion(&self, prefix: &str) -> Vec<String> {
        let mut executables = vec![];

        let paths;
        paths = self.engine_state.env_vars.get("PATH");

        if let Some(paths) = paths {
            if let Ok(paths) = paths.as_list() {
                for path in paths {
                    let path = path.as_string().unwrap_or_default();

                    if let Ok(mut contents) = std::fs::read_dir(path) {
                        while let Some(Ok(item)) = contents.next() {
                            if !executables.contains(
                                &item
                                    .path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                            ) && matches!(
                                item.path()
                                    .file_name()
                                    .map(|x| x.to_string_lossy().starts_with(prefix)),
                                Some(true)
                            ) && is_executable::is_executable(&item.path())
                            {
                                if let Ok(name) = item.file_name().into_string() {
                                    executables.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        executables
    }

    fn complete_variables(
        &self,
        working_set: &StateWorkingSet,
        prefix: &[u8],
        span: Span,
        offset: usize,
    ) -> Vec<(reedline::Span, String)> {
        let mut output = vec![];

        let builtins = ["$nu", "$scope", "$in", "$config", "$env"];

        for builtin in builtins {
            if builtin.as_bytes().starts_with(prefix) {
                output.push((
                    reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    builtin.to_string(),
                ));
            }
        }

        for scope in &working_set.delta.scope {
            for v in &scope.vars {
                if v.0.starts_with(prefix) {
                    output.push((
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        String::from_utf8_lossy(v.0).to_string(),
                    ));
                }
            }
        }
        for scope in &self.engine_state.scope {
            for v in &scope.vars {
                if v.0.starts_with(prefix) {
                    output.push((
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        String::from_utf8_lossy(v.0).to_string(),
                    ));
                }
            }
        }

        output.dedup();

        output
    }

    fn complete_filepath_and_commands(
        &self,
        working_set: &StateWorkingSet,
        span: Span,
        offset: usize,
    ) -> Vec<(reedline::Span, String)> {
        let prefix = working_set.get_span_contents(span);

        let results = working_set
            .find_commands_by_prefix(prefix)
            .into_iter()
            .map(move |x| {
                (
                    reedline::Span {
                        start: span.start - offset,
                        end: span.end - offset,
                    },
                    String::from_utf8_lossy(&x).to_string(),
                )
            });
        let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD") {
            match d.as_string() {
                Ok(s) => s,
                Err(_) => "".to_string(),
            }
        } else {
            "".to_string()
        };

        let prefix = String::from_utf8_lossy(prefix).to_string();
        let results_paths = file_path_completion(span, &prefix, &cwd)
            .into_iter()
            .map(move |x| {
                (
                    reedline::Span {
                        start: x.0.start - offset,
                        end: x.0.end - offset,
                    },
                    x.1,
                )
            });

        let results_external =
            self.external_command_completion(&prefix)
                .into_iter()
                .map(move |x| {
                    (
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        x,
                    )
                });

        results
            .chain(results_paths.into_iter())
            .chain(results_external.into_iter())
            .collect()
    }

    fn completion_helper(&self, line: &str, pos: usize) -> Vec<(reedline::Span, String)> {
        let mut working_set = StateWorkingSet::new(&self.engine_state);
        let offset = working_set.next_span_start();
        let pos = offset + pos;
        let (output, _err) = parse(&mut working_set, Some("completer"), line.as_bytes(), false);

        for stmt in output.stmts.into_iter() {
            if let Statement::Pipeline(pipeline) = stmt {
                for expr in pipeline.expressions {
                    let flattened = flatten_expression(&working_set, &expr);
                    for flat in flattened {
                        if pos >= flat.0.start && pos <= flat.0.end {
                            let prefix = working_set.get_span_contents(flat.0);

                            if prefix.starts_with(b"$") {
                                return self.complete_variables(
                                    &working_set,
                                    prefix,
                                    flat.0,
                                    offset,
                                );
                            }

                            match &flat.1 {
                                nu_parser::FlatShape::Custom(custom_completion) => {
                                    let prefix = working_set.get_span_contents(flat.0).to_vec();

                                    let (block, ..) = parse(
                                        &mut working_set,
                                        None,
                                        custom_completion.as_bytes(),
                                        false,
                                    );

                                    let mut stack = Stack::default();
                                    let result = eval_block(
                                        &self.engine_state,
                                        &mut stack,
                                        &block,
                                        PipelineData::new(flat.0),
                                    );

                                    let v: Vec<_> = match result {
                                        Ok(pd) => pd
                                            .into_iter()
                                            .map(move |x| {
                                                let s = x.as_string().expect(
                                                    "FIXME: better error handling for custom completions",
                                                );

                                                (
                                                    reedline::Span {
                                                        start: flat.0.start - offset,
                                                        end: flat.0.end - offset,
                                                    },
                                                    s,
                                                )
                                            })
                                            .filter(|x| x.1.as_bytes().starts_with(&prefix))
                                            .collect(),
                                        _ => vec![],
                                    };

                                    return v;
                                }
                                nu_parser::FlatShape::External
                                | nu_parser::FlatShape::InternalCall
                                | nu_parser::FlatShape::String => {
                                    return self.complete_filepath_and_commands(
                                        &working_set,
                                        flat.0,
                                        offset,
                                    );
                                }
                                nu_parser::FlatShape::Filepath
                                | nu_parser::FlatShape::GlobPattern
                                | nu_parser::FlatShape::ExternalArg => {
                                    let prefix = working_set.get_span_contents(flat.0);
                                    let prefix = String::from_utf8_lossy(prefix).to_string();
                                    let cwd = if let Some(d) = self.engine_state.env_vars.get("PWD")
                                    {
                                        match d.as_string() {
                                            Ok(s) => s,
                                            Err(_) => "".to_string(),
                                        }
                                    } else {
                                        "".to_string()
                                    };

                                    let results = file_path_completion(flat.0, &prefix, &cwd);

                                    return results
                                        .into_iter()
                                        .map(move |x| {
                                            (
                                                reedline::Span {
                                                    start: x.0.start - offset,
                                                    end: x.0.end - offset,
                                                },
                                                x.1,
                                            )
                                        })
                                        .collect();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        vec![]
    }
}

impl Completer for NuCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<(reedline::Span, String)> {
        let mut output = self.completion_helper(line, pos);

        output.sort_by(|a, b| a.1.cmp(&b.1));

        output
    }
}

fn file_path_completion(
    span: nu_protocol::Span,
    partial: &str,
    cwd: &str,
) -> Vec<(nu_protocol::Span, String)> {
    use std::path::{is_separator, Path};

    let partial = if let Some(s) = partial.strip_prefix('"') {
        s
    } else {
        partial
    };

    let partial = if let Some(s) = partial.strip_suffix('"') {
        s
    } else {
        partial
    };

    let (base_dir_name, partial) = {
        // If partial is only a word we want to search in the current dir
        let (base, rest) = partial.rsplit_once(is_separator).unwrap_or((".", partial));
        // On windows, this standardizes paths to use \
        let mut base = base.replace(is_separator, &SEP.to_string());

        // rsplit_once removes the separator
        base.push(SEP);
        (base, rest)
    };

    let base_dir = nu_path::expand_path_with(&base_dir_name, cwd);
    // This check is here as base_dir.read_dir() with base_dir == "" will open the current dir
    // which we don't want in this case (if we did, base_dir would already be ".")
    if base_dir == Path::new("") {
        return Vec::new();
    }

    if let Ok(result) = base_dir.read_dir() {
        result
            .filter_map(|entry| {
                entry.ok().and_then(|entry| {
                    let mut file_name = entry.file_name().to_string_lossy().into_owned();
                    if matches(partial, &file_name) {
                        let mut path = format!("{}{}", base_dir_name, file_name);
                        if entry.path().is_dir() {
                            path.push(SEP);
                            file_name.push(SEP);
                        }

                        if path.contains(' ') {
                            path = format!("\"{}\"", path);
                        }

                        Some((span, path))
                    } else {
                        None
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

fn matches(partial: &str, from: &str) -> bool {
    from.to_ascii_lowercase()
        .starts_with(&partial.to_ascii_lowercase())
}
