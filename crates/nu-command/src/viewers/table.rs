use lscolors::{LsColors, Style};
use nu_color_config::{get_color_config, style_primitive};
use nu_engine::{column::get_columns, env_to_string, CallExt};
use nu_protocol::{
    ast::{Call, PathMember},
    engine::{Command, EngineState, Stack, StateWorkingSet},
    format_error, Category, Config, DataSource, Example, FooterMode, IntoPipelineData, ListStream,
    PipelineData, PipelineMetadata, RawStream, ShellError, Signature, Span, SyntaxShape, Value,
};
use nu_table::{
    tabled::{
        self,
        object::Object,
        style::{CustomStyle, Symbol},
        Highlight,
    },
    StyledString, TableTheme, TextStyle,
};
use std::sync::Arc;
use std::time::Instant;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
};
use terminal_size::{Height, Width};

//use super::lscolor_ansiterm::ToNuAnsiStyle;

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

fn get_width_param(width_param: Option<i64>) -> usize {
    if let Some(col) = width_param {
        col as usize
    } else if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
        (w - 1) as usize
    } else {
        80usize
    }
}

#[derive(Clone)]
pub struct Table;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn usage(&self) -> &str {
        "Render the table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["display", "render"]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("table")
            .named(
                "start-number",
                SyntaxShape::Int,
                "row number to start viewing from",
                Some('n'),
            )
            .switch("list", "list available table modes/themes", Some('l'))
            .named(
                "width",
                SyntaxShape::Int,
                "number of terminal columns wide (not output columns)",
                Some('w'),
            )
            .category(Category::Viewers)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let config = engine_state.get_config();
        let color_hm = get_color_config(config);
        let start_num: Option<i64> = call.get_flag(engine_state, stack, "start-number")?;
        let row_offset = start_num.unwrap_or_default() as usize;
        let list: bool = call.has_flag("list");

        let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;
        let term_width = get_width_param(width_param);

        if list {
            let table_modes = vec![
                Value::string("basic", Span::test_data()),
                Value::string("compact", Span::test_data()),
                Value::string("compact_double", Span::test_data()),
                Value::string("default", Span::test_data()),
                Value::string("heavy", Span::test_data()),
                Value::string("light", Span::test_data()),
                Value::string("none", Span::test_data()),
                Value::string("reinforced", Span::test_data()),
                Value::string("rounded", Span::test_data()),
                Value::string("thin", Span::test_data()),
                Value::string("with_love", Span::test_data()),
            ];
            return Ok(Value::List {
                vals: table_modes,
                span: Span::test_data(),
            }
            .into_pipeline_data());
        }

        // reset vt processing, aka ansi because illbehaved externals can break it
        #[cfg(windows)]
        {
            let _ = nu_utils::enable_vt_processing();
        }

        match input {
            PipelineData::ExternalStream { .. } => Ok(input),
            PipelineData::Value(Value::Binary { val, .. }, ..) => {
                Ok(PipelineData::ExternalStream {
                    stdout: Some(RawStream::new(
                        Box::new(
                            vec![Ok(format!("{}\n", nu_pretty_hex::pretty_hex(&val))
                                .as_bytes()
                                .to_vec())]
                            .into_iter(),
                        ),
                        ctrlc,
                        head,
                    )),
                    stderr: None,
                    exit_code: None,
                    span: head,
                    metadata: None,
                })
            }
            PipelineData::Value(Value::List { vals, .. }, metadata) => handle_row_stream(
                engine_state,
                stack,
                ListStream::from_stream(vals.into_iter(), ctrlc.clone()),
                call,
                row_offset,
                ctrlc,
                metadata,
            ),
            PipelineData::ListStream(stream, metadata) => handle_row_stream(
                engine_state,
                stack,
                stream,
                call,
                row_offset,
                ctrlc,
                metadata,
            ),
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let mut output = vec![];
                for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                    output.push(vec![
                        use_text_style(c, TextStyle::default_field()),
                        use_text_style(v.into_abbreviated_string(config), TextStyle::default()),
                    ])
                }

                let table = build_table(config, term_width, output, None, None);

                let result = print_table(table, term_width);

                Ok(Value::String {
                    val: result,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::Error { error }, ..) => {
                let working_set = StateWorkingSet::new(engine_state);
                Ok(Value::String {
                    val: format_error(&working_set, &error),
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::CustomValue { val, span }, ..) => {
                let base_pipeline = val.to_base_value(span)?.into_pipeline_data();
                self.run(engine_state, stack, call, base_pipeline)
            }
            PipelineData::Value(Value::Range { val, .. }, metadata) => handle_row_stream(
                engine_state,
                stack,
                ListStream::from_stream(val.into_range_iter(ctrlc.clone())?, ctrlc.clone()),
                call,
                row_offset,
                ctrlc,
                metadata,
            ),
            x => Ok(x),
        }
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "List the files in current directory with index number start from 1.",
                example: r#"ls | table -n 1"#,
                result: None,
            },
            Example {
                description: "Render data in table view",
                example: r#"echo [[a b]; [1 2] [3 4]] | table"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(1), Value::test_int(2)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["a".to_string(), "b".to_string()],
                            vals: vec![Value::test_int(3), Value::test_int(4)],
                            span,
                        },
                    ],
                    span,
                }),
            },
        ]
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_row_stream(
    engine_state: &EngineState,
    stack: &mut Stack,
    stream: ListStream,
    call: &Call,
    row_offset: usize,
    ctrlc: Option<Arc<AtomicBool>>,
    metadata: Option<PipelineMetadata>,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let stream = match metadata {
        Some(PipelineMetadata {
            data_source: DataSource::Ls,
        }) => {
            let config = engine_state.config.clone();
            let ctrlc = ctrlc.clone();

            let ls_colors = match stack.get_env_var(engine_state, "LS_COLORS") {
                            Some(v) => LsColors::from_string(&env_to_string(
                                "LS_COLORS",
                                &v,
                                engine_state,
                                stack,
                            )?),
                            None => LsColors::from_string("st=0:di=0;38;5;81:so=0;38;5;16;48;5;203:ln=0;38;5;203:cd=0;38;5;203;48;5;236:ex=1;38;5;203:or=0;38;5;16;48;5;203:fi=0:bd=0;38;5;81;48;5;236:ow=0:mi=0;38;5;16;48;5;203:*~=0;38;5;243:no=0:tw=0:pi=0;38;5;16;48;5;81:*.z=4;38;5;203:*.t=0;38;5;48:*.o=0;38;5;243:*.d=0;38;5;48:*.a=1;38;5;203:*.c=0;38;5;48:*.m=0;38;5;48:*.p=0;38;5;48:*.r=0;38;5;48:*.h=0;38;5;48:*.ml=0;38;5;48:*.ll=0;38;5;48:*.gv=0;38;5;48:*.cp=0;38;5;48:*.xz=4;38;5;203:*.hs=0;38;5;48:*css=0;38;5;48:*.ui=0;38;5;149:*.pl=0;38;5;48:*.ts=0;38;5;48:*.gz=4;38;5;203:*.so=1;38;5;203:*.cr=0;38;5;48:*.fs=0;38;5;48:*.bz=4;38;5;203:*.ko=1;38;5;203:*.as=0;38;5;48:*.sh=0;38;5;48:*.pp=0;38;5;48:*.el=0;38;5;48:*.py=0;38;5;48:*.lo=0;38;5;243:*.bc=0;38;5;243:*.cc=0;38;5;48:*.pm=0;38;5;48:*.rs=0;38;5;48:*.di=0;38;5;48:*.jl=0;38;5;48:*.rb=0;38;5;48:*.md=0;38;5;185:*.js=0;38;5;48:*.go=0;38;5;48:*.vb=0;38;5;48:*.hi=0;38;5;243:*.kt=0;38;5;48:*.hh=0;38;5;48:*.cs=0;38;5;48:*.mn=0;38;5;48:*.nb=0;38;5;48:*.7z=4;38;5;203:*.ex=0;38;5;48:*.rm=0;38;5;208:*.ps=0;38;5;186:*.td=0;38;5;48:*.la=0;38;5;243:*.aux=0;38;5;243:*.xmp=0;38;5;149:*.mp4=0;38;5;208:*.rpm=4;38;5;203:*.m4a=0;38;5;208:*.zip=4;38;5;203:*.dll=1;38;5;203:*.bcf=0;38;5;243:*.awk=0;38;5;48:*.aif=0;38;5;208:*.zst=4;38;5;203:*.bak=0;38;5;243:*.tgz=4;38;5;203:*.com=1;38;5;203:*.clj=0;38;5;48:*.sxw=0;38;5;186:*.vob=0;38;5;208:*.fsx=0;38;5;48:*.doc=0;38;5;186:*.mkv=0;38;5;208:*.tbz=4;38;5;203:*.ogg=0;38;5;208:*.wma=0;38;5;208:*.mid=0;38;5;208:*.kex=0;38;5;186:*.out=0;38;5;243:*.ltx=0;38;5;48:*.sql=0;38;5;48:*.ppt=0;38;5;186:*.tex=0;38;5;48:*.odp=0;38;5;186:*.log=0;38;5;243:*.arj=4;38;5;203:*.ipp=0;38;5;48:*.sbt=0;38;5;48:*.jpg=0;38;5;208:*.yml=0;38;5;149:*.txt=0;38;5;185:*.csv=0;38;5;185:*.dox=0;38;5;149:*.pro=0;38;5;149:*.bst=0;38;5;149:*TODO=1:*.mir=0;38;5;48:*.bat=1;38;5;203:*.m4v=0;38;5;208:*.pod=0;38;5;48:*.cfg=0;38;5;149:*.pas=0;38;5;48:*.tml=0;38;5;149:*.bib=0;38;5;149:*.ini=0;38;5;149:*.apk=4;38;5;203:*.h++=0;38;5;48:*.pyc=0;38;5;243:*.img=4;38;5;203:*.rst=0;38;5;185:*.swf=0;38;5;208:*.htm=0;38;5;185:*.ttf=0;38;5;208:*.elm=0;38;5;48:*hgrc=0;38;5;149:*.bmp=0;38;5;208:*.fsi=0;38;5;48:*.pgm=0;38;5;208:*.dpr=0;38;5;48:*.xls=0;38;5;186:*.tcl=0;38;5;48:*.mli=0;38;5;48:*.ppm=0;38;5;208:*.bbl=0;38;5;243:*.lua=0;38;5;48:*.asa=0;38;5;48:*.pbm=0;38;5;208:*.avi=0;38;5;208:*.def=0;38;5;48:*.mov=0;38;5;208:*.hxx=0;38;5;48:*.tif=0;38;5;208:*.fon=0;38;5;208:*.zsh=0;38;5;48:*.png=0;38;5;208:*.inc=0;38;5;48:*.jar=4;38;5;203:*.swp=0;38;5;243:*.pid=0;38;5;243:*.gif=0;38;5;208:*.ind=0;38;5;243:*.erl=0;38;5;48:*.ilg=0;38;5;243:*.eps=0;38;5;208:*.tsx=0;38;5;48:*.git=0;38;5;243:*.inl=0;38;5;48:*.rtf=0;38;5;186:*.hpp=0;38;5;48:*.kts=0;38;5;48:*.deb=4;38;5;203:*.svg=0;38;5;208:*.pps=0;38;5;186:*.ps1=0;38;5;48:*.c++=0;38;5;48:*.cpp=0;38;5;48:*.bsh=0;38;5;48:*.php=0;38;5;48:*.exs=0;38;5;48:*.toc=0;38;5;243:*.mp3=0;38;5;208:*.epp=0;38;5;48:*.rar=4;38;5;203:*.wav=0;38;5;208:*.xlr=0;38;5;186:*.tmp=0;38;5;243:*.cxx=0;38;5;48:*.iso=4;38;5;203:*.dmg=4;38;5;203:*.gvy=0;38;5;48:*.bin=4;38;5;203:*.wmv=0;38;5;208:*.blg=0;38;5;243:*.ods=0;38;5;186:*.psd=0;38;5;208:*.mpg=0;38;5;208:*.dot=0;38;5;48:*.cgi=0;38;5;48:*.xml=0;38;5;185:*.htc=0;38;5;48:*.ics=0;38;5;186:*.bz2=4;38;5;203:*.tar=4;38;5;203:*.csx=0;38;5;48:*.ico=0;38;5;208:*.sxi=0;38;5;186:*.nix=0;38;5;149:*.pkg=4;38;5;203:*.bag=4;38;5;203:*.fnt=0;38;5;208:*.idx=0;38;5;243:*.xcf=0;38;5;208:*.exe=1;38;5;203:*.flv=0;38;5;208:*.fls=0;38;5;243:*.otf=0;38;5;208:*.vcd=4;38;5;203:*.vim=0;38;5;48:*.sty=0;38;5;243:*.pdf=0;38;5;186:*.odt=0;38;5;186:*.purs=0;38;5;48:*.h264=0;38;5;208:*.jpeg=0;38;5;208:*.dart=0;38;5;48:*.pptx=0;38;5;186:*.lock=0;38;5;243:*.bash=0;38;5;48:*.rlib=0;38;5;243:*.hgrc=0;38;5;149:*.psm1=0;38;5;48:*.toml=0;38;5;149:*.tbz2=4;38;5;203:*.yaml=0;38;5;149:*.make=0;38;5;149:*.orig=0;38;5;243:*.html=0;38;5;185:*.fish=0;38;5;48:*.diff=0;38;5;48:*.xlsx=0;38;5;186:*.docx=0;38;5;186:*.json=0;38;5;149:*.psd1=0;38;5;48:*.tiff=0;38;5;208:*.flac=0;38;5;208:*.java=0;38;5;48:*.less=0;38;5;48:*.mpeg=0;38;5;208:*.conf=0;38;5;149:*.lisp=0;38;5;48:*.epub=0;38;5;186:*.cabal=0;38;5;48:*.patch=0;38;5;48:*.shtml=0;38;5;185:*.class=0;38;5;243:*.xhtml=0;38;5;185:*.mdown=0;38;5;185:*.dyn_o=0;38;5;243:*.cache=0;38;5;243:*.swift=0;38;5;48:*README=0;38;5;16;48;5;186:*passwd=0;38;5;149:*.ipynb=0;38;5;48:*shadow=0;38;5;149:*.toast=4;38;5;203:*.cmake=0;38;5;149:*.scala=0;38;5;48:*.dyn_hi=0;38;5;243:*.matlab=0;38;5;48:*.config=0;38;5;149:*.gradle=0;38;5;48:*.groovy=0;38;5;48:*.ignore=0;38;5;149:*LICENSE=0;38;5;249:*TODO.md=1:*COPYING=0;38;5;249:*.flake8=0;38;5;149:*INSTALL=0;38;5;16;48;5;186:*setup.py=0;38;5;149:*.gemspec=0;38;5;149:*.desktop=0;38;5;149:*Makefile=0;38;5;149:*Doxyfile=0;38;5;149:*TODO.txt=1:*README.md=0;38;5;16;48;5;186:*.kdevelop=0;38;5;149:*.rgignore=0;38;5;149:*configure=0;38;5;149:*.DS_Store=0;38;5;243:*.fdignore=0;38;5;149:*COPYRIGHT=0;38;5;249:*.markdown=0;38;5;185:*.cmake.in=0;38;5;149:*.gitconfig=0;38;5;149:*INSTALL.md=0;38;5;16;48;5;186:*CODEOWNERS=0;38;5;149:*.gitignore=0;38;5;149:*Dockerfile=0;38;5;149:*SConstruct=0;38;5;149:*.scons_opt=0;38;5;243:*README.txt=0;38;5;16;48;5;186:*SConscript=0;38;5;149:*.localized=0;38;5;243:*.travis.yml=0;38;5;186:*Makefile.in=0;38;5;243:*.gitmodules=0;38;5;149:*LICENSE-MIT=0;38;5;249:*Makefile.am=0;38;5;149:*INSTALL.txt=0;38;5;16;48;5;186:*MANIFEST.in=0;38;5;149:*.synctex.gz=0;38;5;243:*.fdb_latexmk=0;38;5;243:*CONTRIBUTORS=0;38;5;16;48;5;186:*configure.ac=0;38;5;149:*.applescript=0;38;5;48:*appveyor.yml=0;38;5;186:*.clang-format=0;38;5;149:*.gitattributes=0;38;5;149:*LICENSE-APACHE=0;38;5;249:*CMakeCache.txt=0;38;5;243:*CMakeLists.txt=0;38;5;149:*CONTRIBUTORS.md=0;38;5;16;48;5;186:*requirements.txt=0;38;5;149:*CONTRIBUTORS.txt=0;38;5;16;48;5;186:*.sconsign.dblite=0;38;5;243:*package-lock.json=0;38;5;243:*.CFUserTextEncoding=0;38;5;243"),
                        };

            ListStream::from_stream(
                stream.map(move |mut x| match &mut x {
                    Value::Record { cols, vals, .. } => {
                        let mut idx = 0;

                        while idx < cols.len() {
                            if cols[idx] == "name" {
                                if let Some(Value::String { val: path, span }) = vals.get(idx) {
                                    match std::fs::symlink_metadata(&path) {
                                        Ok(metadata) => {
                                            let style = ls_colors.style_for_path_with_metadata(
                                                path.clone(),
                                                Some(&metadata),
                                            );
                                            let ansi_style = style
                                                .map(Style::to_crossterm_style)
                                                // .map(ToNuAnsiStyle::to_nu_ansi_style)
                                                .unwrap_or_default();
                                            let use_ls_colors = config.use_ls_colors;

                                            if use_ls_colors {
                                                vals[idx] = Value::String {
                                                    val: ansi_style.apply(path).to_string(),
                                                    span: *span,
                                                };
                                            }
                                        }
                                        Err(_) => {
                                            let style = ls_colors.style_for_path(path.clone());
                                            let ansi_style = style
                                                .map(Style::to_crossterm_style)
                                                // .map(ToNuAnsiStyle::to_nu_ansi_style)
                                                .unwrap_or_default();
                                            let use_ls_colors = config.use_ls_colors;

                                            if use_ls_colors {
                                                vals[idx] = Value::String {
                                                    val: ansi_style.apply(path).to_string(),
                                                    span: *span,
                                                };
                                            }
                                        }
                                    }
                                }
                            }

                            idx += 1;
                        }

                        x
                    }
                    _ => x,
                }),
                ctrlc,
            )
        }
        _ => stream,
    };

    let head = call.head;
    let width_param: Option<i64> = call.get_flag(engine_state, stack, "width")?;

    Ok(PipelineData::ExternalStream {
        stdout: Some(RawStream::new(
            Box::new(PagingTableCreator {
                row_offset,
                config: engine_state.get_config().clone(),
                ctrlc: ctrlc.clone(),
                head,
                stream,
                width_param,
            }),
            ctrlc,
            head,
        )),
        stderr: None,
        exit_code: None,
        span: head,
        metadata: None,
    })
}

fn convert_data(
    row_offset: usize,
    input: &[Value],
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
    head: Span,
) -> Result<Option<(Vec<Vec<String>>, Vec<String>, Vec<Vec<nu_table::Alignment>>)>, ShellError> {
    let mut headers = get_columns(input);
    let mut input = input.iter().peekable();
    let color_hm = get_color_config(config);
    let float_precision = config.float_precision as usize;
    let disable_index = config.disable_table_indexes;

    if input.peek().is_none() {
        return Ok(None);
    }

    if !headers.is_empty() && !disable_index {
        headers.insert(0, "#".into());
    }

    // Vec of Vec of String1, String2 where String1 is datatype and String2 is value
    let mut data: Vec<Vec<(String, String)>> = Vec::new();

    for (row_num, item) in input.enumerate() {
        if let Some(ctrlc) = &ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                return Ok(None);
            }
        }
        if let Value::Error { error } = item {
            return Err(error.clone());
        }
        // String1 = datatype, String2 = value as string
        let mut row: Vec<(String, String)> = vec![];
        if !disable_index {
            row = vec![("string".to_string(), (row_num + row_offset).to_string())];
        }

        if headers.is_empty() {
            row.push((
                item.get_type().to_string(),
                item.into_abbreviated_string(config),
            ));
        } else {
            let skip_num = if !disable_index { 1 } else { 0 };
            for header in headers.iter().skip(skip_num) {
                let result = match item {
                    Value::Record { .. } => item.clone().follow_cell_path(&[PathMember::String {
                        val: header.into(),
                        span: head,
                    }]),
                    _ => Ok(item.clone()),
                };

                match result {
                    Ok(value) => row.push((
                        (&value.get_type()).to_string(),
                        value.into_abbreviated_string(config),
                    )),
                    Err(_) => row.push(("empty".to_string(), "❎".into())),
                }
            }
        }

        data.push(row);
    }

    let alignment_map = data
        .iter()
        .map(|x| {
            x.iter()
                .enumerate()
                .map(|(col, y)| {
                    if col == 0 && !disable_index {
                        nu_table::Alignment::Right
                    } else {
                        get_primitive_alignment(&y.0, &color_hm)
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let data = data
        .into_iter()
        .map(|x| {
            x.into_iter()
                .enumerate()
                .map(|(col, y)| {
                    if col == 0 && !disable_index {
                        color_hm["row_index"].paint(y.1).to_string()
                    } else if &y.0 == "float" {
                        // set dynamic precision from config
                        let precise_number = convert_with_precision(&y.1, float_precision)
                            .unwrap_or_else(|e| e.to_string());

                        use_primitive_style(precise_number, &y.0, &color_hm)
                    } else {
                        use_primitive_style(y.1, &y.0, &color_hm)
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let headers = headers
        .into_iter()
        .map(|s| color_hm["header"].paint(s).to_string())
        .collect::<Vec<_>>();

    Ok(Some((data, headers, alignment_map)))
}

fn use_primitive_style(
    text: String,
    primitive: &str,
    color_hm: &std::collections::HashMap<String, nu_ansi_term::Style>,
) -> String {
    let style = style_primitive(primitive, color_hm);
    match style.color_style {
        Some(s) => s.paint(text).to_string(),
        None => text,
    }
}

fn use_text_style(text: String, style: TextStyle) -> String {
    match style.color_style {
        Some(s) => s.paint(text).to_string(),
        None => text,
    }
}

fn get_primitive_alignment(
    primitive: &str,
    color_hm: &std::collections::HashMap<String, nu_ansi_term::Style>,
) -> nu_table::Alignment {
    let style = style_primitive(primitive, color_hm);
    style.alignment
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError(
                format!("error converting string [{}] to f64", &val),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            ));
        }
    };
    Ok(format!("{:.prec$}", val_float, prec = precision))
}

struct PagingTableCreator {
    head: Span,
    stream: ListStream,
    ctrlc: Option<Arc<AtomicBool>>,
    config: Config,
    row_offset: usize,
    width_param: Option<i64>,
}

impl Iterator for PagingTableCreator {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = vec![];

        let start_time = Instant::now();

        let mut idx = 0;

        // Pull from stream until time runs out or we have enough items
        for item in self.stream.by_ref() {
            batch.push(item);
            idx += 1;

            if idx % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                let end_time = Instant::now();

                // If we've been buffering over a second, go ahead and send out what we have so far
                if (end_time - start_time).as_secs() >= 1 {
                    break;
                }
            }

            if idx == STREAM_PAGE_SIZE {
                break;
            }

            if let Some(ctrlc) = &self.ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    break;
                }
            }
        }

        let table = convert_data(
            self.row_offset,
            &batch,
            self.ctrlc.clone(),
            &self.config,
            self.head,
        );
        self.row_offset += idx;

        let term_width = get_width_param(self.width_param);

        match table {
            Ok(Some((data, headers, alignment_map))) => {
                let table = build_table(
                    &self.config,
                    term_width,
                    data,
                    Some(headers),
                    Some(alignment_map),
                );

                Some(Ok(print_table(table, term_width).as_bytes().to_vec()))
            }
            Err(err) => Some(Err(err)),
            _ => None,
        }
    }
}

fn print_table(mut table: tabled::Table, term_width: usize) -> String {
    let mut width = CalculateTableWidth(0);
    table = table.with(&mut width);
    if width.0 > term_width {
        return format!("Couldn't fit table into {} columns!", term_width);
    }

    table.to_string()
}

fn build_table(
    config: &Config,
    term_width: usize,
    data: Vec<Vec<String>>,
    headers: Option<Vec<String>>,
    alignment_map: Option<Vec<Vec<nu_table::Alignment>>>,
) -> tabled::Table {
    let count_records = data.len();
    let header_present = headers.is_some();
    let mut builder = tabled::builder::Builder::from(data);

    if let Some(headers) = headers {
        builder = add_footer(config, count_records as u64, &headers, builder);
        builder = builder.set_columns(headers)
    }

    let mut table = builder.build();

    table = table.with(
        tabled::Modify::new(tabled::object::Segment::all())
            .with(tabled::Width::truncate(config.truncate_table_strings_at as usize).suffix("..")),
    );

    table = load_theme_from_config(config, table, header_present)
        .with(tabled::Width::wrap(term_width).priority::<tabled::width::PriorityMax>())
        .with(tabled::Modify::new(tabled::object::Rows::new(1..)).with(tabled::Alignment::left()));

    if !config.disable_table_indexes {
        table = table.with(
            tabled::Modify::new(tabled::object::Columns::first()).with(tabled::Alignment::right()),
        );
    }

    if header_present {
        table = table.with(
            tabled::Modify::new(tabled::object::Rows::first()).with(tabled::Alignment::center()),
        );

        if need_footer(config, count_records as u64) {
            table = table.with(FooterStyle);
            table = table.with(
                tabled::Modify::new(tabled::object::Rows::last()).with(tabled::Alignment::center()),
            );
        }
    }

    if let Some(alignment) = alignment_map {
        let offset = if header_present { 1 } else { 0 };
        for (row, alignments) in alignment.into_iter().enumerate() {
            for (col, alignment) in alignments.into_iter().enumerate() {
                let alignment = nu_table_alignment_to_tabled_alignment(alignment);
                table = table.with(
                    tabled::Modify::new(tabled::object::Cell(row + offset, col)).with(alignment),
                );
            }
        }
    }

    table
}

fn nu_table_alignment_to_tabled_alignment(alignment: nu_table::Alignment) -> tabled::Alignment {
    match alignment {
        nu_table::Alignment::Left => tabled::Alignment::left(),
        nu_table::Alignment::Center => tabled::Alignment::center(),
        nu_table::Alignment::Right => tabled::Alignment::right(),
    }
}

fn load_theme_from_config(
    config: &Config,
    mut table: tabled::Table,
    with_header: bool,
) -> tabled::Table {
    let mut style: tabled::style::StyleSettings = match config.table_mode.as_str() {
        "basic" => tabled::Style::ascii().into(),
        "compact" => tabled::Style::modern().into(),
        "compact_double" => tabled::Style::extended().into(),
        "light" => tabled::Style::psql().into(),
        "with_love" => tabled::Style::blank()
            .left(' ')
            .top(' ')
            .bottom(' ')
            .top_left_corner('❤')
            .bottom_left_corner('❤')
            .into(),
        "rounded" => tabled::Style::rounded().into(),
        "reinforced" => tabled::Style::re_structured_text().into(),
        "heavy" => tabled::Style::github_markdown().into(),
        "none" => tabled::Style::blank().into(),
        _ => tabled::Style::rounded().into(),
    };

    let color_hm = get_color_config(config);
    if let Some(color) = color_hm.get("separator") {
        style = style.try_map(|s| Symbol::ansi(color.paint(s.to_string()).to_string()).unwrap());
    }

    table = table.with(style);

    if !with_header {
        table = table.with(RemoveHeaderLine);
    }

    table
}

fn add_footer(
    config: &Config,
    count_records: u64,
    headers: &[String],
    mut b: tabled::builder::Builder,
) -> tabled::builder::Builder {
    let insert_footer = need_footer(config, count_records);
    if insert_footer {
        b = b.add_record(headers);
    }

    b
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}

struct FooterStyle;

impl tabled::TableOption for FooterStyle {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        if grid.count_columns() == 0 || grid.count_rows() == 0 {
            return;
        }

        let mut line = tabled::papergrid::Line::default();

        let border = grid.get_border(0, 0);
        line.left = border.left_bottom_corner;
        line.intersection = border.right_bottom_corner;
        line.horizontal = border.bottom;

        let border = grid.get_border(0, grid.count_columns() - 1);
        line.right = border.right_bottom_corner;

        grid.set_split_line(grid.count_rows() - 1, line);
    }
}

struct CalculateTableWidth(usize);

impl tabled::TableOption for CalculateTableWidth {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        self.0 = grid.total_width();
    }
}

struct RemoveHeaderLine;

impl tabled::TableOption for RemoveHeaderLine {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        grid.set_split_line(1, tabled::papergrid::Line::default());
    }
}
