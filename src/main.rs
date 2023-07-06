use std::borrow::Cow::{self, Borrowed, Owned};
use std::env;
use std::io::{stdout};
use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use colored::*;

#[cfg(windows)]
use winreg::{enums::*, RegKey};

use crossterm::{execute, terminal::{ScrollUp, SetSize}};
use rustyline::completion::FilenameCompleter;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};
use rustyline::{Completer, Helper, Hinter, Validator};
use std::io::BufRead;

#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

fn main() -> rustyline::Result<()> {
    // let (cols, rows) = size().unwrap();
    // Resize terminal and scroll up.
    execute!(stdout(), SetSize(150, 50), ScrollUp(5)).unwrap();

    // Set the current directory to the home directory
    if let Some(home_dir) = env::home_dir() {
        env::set_current_dir(home_dir).unwrap();
    }

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .build();

    print_startup_banner();

    let h = MyHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter{},
        colored_prompt: "".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);

    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    loop {
        let p = format!("{}>> ", env::current_dir().unwrap().to_str().expect("no current dir"));
        rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{p}\x1b[0m");

        let readline = rl.readline(&p);

        match readline {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                rl.add_history_entry(line.as_str())?;

                let parts: Vec<&str> = line.split_whitespace().collect();

                match parts[0] {
                    "exit" => break,
                    "pwd" => println!("{}", env::current_dir().unwrap().display()),
                    "cd" => {
                        let new_dir = parts.get(1).map_or("/", |s| *s);
                        let root = Path::new(new_dir);

                        if let Err(e) = env::set_current_dir(&root) {
                            eprintln!("{}", e);
                        }
                    },
                    "ls" => {
                        let output = fs::read_dir(".")?;
                        for entry in output {
                            let entry = entry?;
                            let file_name = entry.file_name();
                            println!("{}", file_name.to_string_lossy());
                        }
                    },
                    "ssh" => {
                        if let Some(host) = parts.get(1) {
                            ssh(host);
                        } else {
                            eprintln!("usage: ssh ");
                        }
                    },
                    command => {
                        if let Some(executable_path) = find_executable(command) {
                            let mut child = Command::new(executable_path)
                                .args(&parts[1..])
                                .stdin(std::process::Stdio::inherit())
                                .stdout(std::process::Stdio::inherit())
                                .stderr(std::process::Stdio::inherit())
                                .spawn()
                                .expect("failed to execute process");

                            child.wait().expect("failed to wait on child process");
                        } else {
                            eprintln!("unknown command: {}", command);
                        }
                    },
                };
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            },
        }
    }

    rl.append_history("history.txt")
}

fn print_startup_banner() { let banner = r#" 
██╗    ██╗███████╗██╗      ██████╗ ██████╗ ███╗   ███╗███████╗    ████████╗ ██████╗    
██║    ██║██╔════╝██║     ██╔════╝██╔═══██╗████╗ ████║██╔════╝    ╚══██╔══╝██╔═══██╗   
██║ █╗ ██║█████╗  ██║     ██║     ██║   ██║██╔████╔██║█████╗         ██║   ██║   ██║   
██║███╗██║██╔══╝  ██║     ██║     ██║   ██║██║╚██╔╝██║██╔══╝         ██║   ██║   ██║   
╚███╔███╔╝███████╗███████╗╚██████╗╚██████╔╝██║ ╚═╝ ██║███████╗       ██║   ╚██████╔╝   
 ╚══╝╚══╝ ╚══════╝╚══════╝ ╚═════╝ ╚═════╝ ╚═╝     ╚═╝╚══════╝       ╚═╝    ╚═════╝    
"#;                                                                                       

                                                                                       
let banner2 = r#" 
██████╗ ██╗   ██╗███████╗████████╗██╗   ██╗    ███████╗██╗  ██╗███████╗██╗     ██╗     
██╔══██╗██║   ██║██╔════╝╚══██╔══╝╚██╗ ██╔╝    ██╔════╝██║  ██║██╔════╝██║     ██║     
██████╔╝██║   ██║███████╗   ██║    ╚████╔╝     ███████╗███████║█████╗  ██║     ██║     
██╔══██╗██║   ██║╚════██║   ██║     ╚██╔╝      ╚════██║██╔══██║██╔══╝  ██║     ██║     
██║  ██║╚██████╔╝███████║   ██║      ██║       ███████║██║  ██║███████╗███████╗███████╗
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝      ╚═╝       ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝
Welcome to the MLP SDP Rusty Shell! 
Type 'exit' to exit. "#; 

println!("{}", banner.color("cyan")); println!("{}", banner2.color("magenta")); 
}

fn ssh(host: &str) {
    let status = Command::new("ssh")
        .arg(host)
        .status()
        .expect("failed to execute ssh");

    if !status.success() {
        eprintln!("ssh exited with non-zero status: {}", status);
    }
}

fn find_executable(executable: &str) -> Option<PathBuf> {
    let exe_suffixes = if cfg!(windows) {
        vec![env::consts::EXE_SUFFIX, ".cmd"]
    } else {
        vec![env::consts::EXE_SUFFIX]
    };

    let mut search_dirs = vec![env::current_dir().unwrap()];

    if let Some(path_var) = env::var("PATH").ok() {
        search_dirs.extend(env::split_paths(&path_var));
    }

    for dir in search_dirs {
        for suffix in &exe_suffixes {
            let exe_name = format!("{}{}", executable, suffix);
            let path = dir.join(&exe_name);

            if path.exists() && fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false) {
                return Some(path);
            }
        }
    }

    None
}
