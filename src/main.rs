use std::{
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
};
use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use ignore::{ParallelVisitor, ParallelVisitorBuilder};
use tokio::{
    task::{JoinError, spawn_blocking},
    time::error::Elapsed,
};

static CANCEL: AtomicBool = AtomicBool::new(false);

fn run_searcher(path: PathBuf) -> Vec<PathBuf> {
    struct Builder<'a>(&'a Mutex<Vec<PathBuf>>);
    struct Visitor<'a> {
        storage: Vec<PathBuf>,
        output: &'a Mutex<Vec<PathBuf>>,
    }

    impl<'a> Drop for Visitor<'a> {
        fn drop(&mut self) {
            self.output
                .lock()
                .expect("Output mutex was poisoned")
                .append(&mut self.storage);
        }
    }

    impl<'s, 'a: 's> ParallelVisitorBuilder<'s> for Builder<'a> {
        fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
            Box::new(Visitor {
                storage: Vec::new(),
                output: self.0,
            })
        }
    }

    impl<'a> ParallelVisitor for Visitor<'a> {
        fn visit(&mut self, entry: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
            if let Ok(entry) = entry {
                if let Some(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        self.storage.push(entry.into_path());
                    }
                }
            }
            ignore::WalkState::Continue
        }
    }

    let output: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
    ignore::WalkBuilder::new(path)
        .follow_links(false)
        .filter_entry(|p| {
            !CANCEL.load(Ordering::Relaxed)
                && p.file_type().is_some_and(|v| {
                    v.is_dir()
                        || (v.is_file() && p.path().extension().is_some_and(|e| e == "kicad_pro"))
                })
        })
        .build_parallel()
        .visit(&mut Builder(&output));
    output
        .into_inner()
        .expect("Shared output mutex was poisoned")
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Timeout while searching for .kicad_pro files")]
    Timeout(#[from] Elapsed),
    #[error("Could not join thread searching for .kicad_pro files")]
    JoinError(#[from] JoinError),
    #[error("No .kicad_pro files found in directory {}", (.0).display())]
    NoProjectsFound(PathBuf),
    #[error("Multiple .kicad_pro files:\n{}", (.0).iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join("\n"))]
    MultipleProjectsFound(Vec<PathBuf>),
}

async fn find_kicad_project(path: PathBuf) -> Result<PathBuf, Error> {
    let handle = spawn_blocking({
        let path = path.clone();
        move || run_searcher(path)
    });
    let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
    CANCEL.store(true, Ordering::Relaxed);

    let mut v = result??;

    if v.is_empty() {
        Err(Error::NoProjectsFound(path))
    } else if v.len() == 1 {
        Ok(v.pop().unwrap())
    } else {
        Err(Error::MultipleProjectsFound(v))
    }
}

fn exec_kicad<const N: usize>(args: [PathBuf; N]) -> Result<()> {
    Command::new("kicad")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn clap_v3_styling() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Green.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Debug, Parser)]
#[command(styles = clap_v3_styling())]
struct App {
    #[clap(long, short = 'r', conflicts_with_all(&["path"]))]
    recent: bool,
    path: Option<PathBuf>,
}

async fn run(path: PathBuf) -> Result<()> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "kicad_pro") {
            exec_kicad([path])
        } else {
            bail!(
                "Not a kicad project file or a directory: {}",
                path.display()
            );
        }
    } else if path.is_dir() {
        let project = find_kicad_project(
            std::env::current_dir().wrap_err("Unable to get current directory")?,
        )
        .await?;
        exec_kicad([project])
    } else {
        bail!("Not a kicad project file or directory: {}", path.display());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let app = App::parse();

    if let Some(path) = app.path {
        run(path).await
    } else if app.recent {
        exec_kicad([])
    } else {
        run(std::env::current_dir()?).await
    }
}
