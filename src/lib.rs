//! MHgit is a simple git library for interracting with git repositories.
//! 
//! Interraction with git repositories are done through [`Repository`] objects.
//! 
//! Simple git commands can be run with the repository methods. For more 
//! complex commands, or to set command options before running, several 
//! _Options_ types are provided.
//! 
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mhgit::{Repository, CommandOptions};
//! use mhgit::commands::AddOptions;
//! 
//! let repo = Repository::new();
//! AddOptions::new()
//!            .all(true)
//!            .run(&repo)?;
//! # Ok(())
//! # }
//! ```
//! 
//! Creating repository
//! -------------------
//! 
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use mhgit::Repository;
//! Repository::at("/home/mh/awesomeness")?
//!            .init()?
//!            .add()?
//!            .commit("Initial commit")?;
//! # Ok(())
//! # }
//! ```
//! 
//! [`Repository`]: struct.Repository.html

// Copyright 2020 Magnus Aa. Hirth. All rights reserved.

#![allow(unused_imports, unused_variables, dead_code)]

#[macro_use]
extern crate failure;

use failure::{Fail, ResultExt};
use std::convert::TryFrom;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio, Output};

mod status;
pub mod commands;

pub use status::Status;

type Result<T> = std::result::Result<T, failure::Error>;

/// Git errors are returned when a git command fails.
#[derive(Fail, Debug)]
pub struct GitError {
    cmd: String,
    code: Option<i32>,
    #[cause] stderr: failure::Error,
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(code) = self.code {
            write!(f, "{} returned error code {}", self.cmd, code)
        } else {
            write!(f, "{} was stopped...", self.cmd)
        }
    }
}

/// GitOut indicates if git output should be piped or printed.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum GitOut {
    Print,
    Pipe,
}

impl Default for GitOut {
    fn default() -> Self {
        GitOut::Pipe
    }
}

/// A handle to a git repository.
/// 
/// By creating with [`at`] the repository may be somewhere other than in
/// current working directory. 
/// 
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::Repository;
/// Repository::at("/home/mh/awesomeness")?
///            .init()?
///            .add()?
///            .commit("Initial commit")?;
/// # Ok(())
/// # }
/// ```
/// 
/// [`at`]: struct.Repository.html#method.at
#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct Repository {
    // Location of repository.
    location: Option<PathBuf>,
    stdout: GitOut,
}

/// Trait implemented by all command option struct ([`CommitOptions`], [`PushOptions`], etc.)
/// 
/// [`CommitOptions`]: commands/struct.CommitOptions.html
/// [`PushOptions`]: commands/struct.PushOptions.html
pub trait CommandOptions {
    type Output;

    /// Return a vector of the arguments passed to git. 
    /// 
    /// The vector contains at least one element, which is the name of the subcommand.
    fn git_args(&self) -> Vec<&str>;

    /// Parse the captured stdout into an appropriate rust type.
    fn parse_output(&self, out: &str) -> Result<Self::Output>;

    /// Run the command in the given git repository. Calling git and parsing
    /// and return the output of the command, if any.
    /// 
    /// If the git command returns error a GitError is returned.
    /// 
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use mhgit::{Repository, CommandOptions};
    /// use mhgit::commands::AddOptions;
    /// let repo = Repository::new();
    /// let _ = AddOptions::new()
    ///                    .all(true)
    ///                    .run(&repo)?;
    /// # Ok(())
    /// # }
    /// ```
    /// 
    fn run(&self, repo: &Repository) -> Result<Self::Output> {
        let args = self.git_args();
        let out = repo.run(args)?;
        self.parse_output(&out)
    }
}

impl Repository {

    /// Get a repository in the current directory.
    /// 
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use mhgit::Repository;
    /// let status = Repository::new()
    ///                         .status()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Repository {
        // TODO: check if git is installed on the system
        Repository {
            ..Default::default()
        }
    }

    /// Get a repository at the given location.
    /// 
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use mhgit::Repository;
    /// Repository::at("/home/mh/awesomeness")?
    ///            .init()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn at<P: AsRef<Path>>(path: P) -> Result<Repository> {
        Ok(Repository {
            location: Some(
                fs::canonicalize(path).context("failed to canonicalize repository path")?,
            ),
            ..Default::default()
        })
    }

    /// Return true if the repository is initialized.
    pub fn is_init(&self) -> bool {
        let git_dir = match &self.location {
            Some(loc) => loc.join(".git"),
            None      => PathBuf::from("./.git"),
        };
        git_dir.exists() && git_dir.is_dir()
    }

    /// Configure if the output of git commands run in this repo should be
    /// piped or printed to screen. 
    /// 
    /// Piping is default. 
    /// 
    /// ```rust,no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use mhgit::Repository;
    /// use mhgit::GitOut::Print;
    /// Repository::new()
    ///     .gitout(Print)
    ///     .status()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn gitout(&mut self, val: GitOut) -> &mut Repository {
        self.stdout = val;
        self
    }

    /// Run `git add` in the repository.
    /// 
    /// The command is called with the --all option. To call `git add` with
    /// different options use [`AddOptions`].
    /// 
    /// [`AddOptions`]: commands/struct.AddOptions.html
    pub fn add(&mut self) -> Result<&mut Self> {
        let args = vec!["add", "--all"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git commit` in the repository, with the given commit message.
    /// 
    /// The command is called with --allow-empty, avoiding errors if no changes
    /// were added since last commit. To call `git commit` with different
    /// options use [`CommitOptions`].
    /// 
    /// [`CommitOptions`]: commands/struct.CommitOptions.html
    pub fn commit(&mut self, msg: &str) -> Result<&mut Self> {
        let args = vec!["commit", "-m", msg, "-q", "--allow-empty"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git fetch` in the repository.
    /// 
    /// The command is called with --all
    pub fn fetch(&mut self) -> Result<&mut Self> {
        let args = vec!["fetch", "--all", "-q"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git init`, initializing the repository.
    pub fn init(&mut self) -> Result<&mut Self> {
        // Create the directory if it doesn't already exist
        if let Some(loc) = &self.location {
            if !loc.exists() {
                fs::create_dir_all(loc)?;
            }
        }
        let args = vec!["init", "-q"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git notes add`, adding a note to HEAD.
    /// 
    /// To call `git notes` with different optinos use [`NotesOptions`].
    /// 
    /// [`NotesOptions`]: commands/struct.NotesOptions.html
    pub fn notes(&mut self, msg: &str) -> Result<&mut Self> {
        let args = vec!["notes", "add", "-m", msg];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git pull` without specifying remote or refs.
    /// 
    /// To call `git pull` with different options use [`PullOptions`].
    /// 
    /// [`PullOptions`]: commands/struct.PullOptions.html
    pub fn pull(&mut self) -> Result<&mut Self> {
        let args = vec!["pull", "-q"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git push` without specifying remote or refs.
    /// 
    /// To call `git push` with different options use [`PushOptions`].
    /// 
    /// [`PushOptions`]: commands/struct.PushOptions.html
    pub fn push(&mut self) -> Result<&mut Self> {
        let args = vec!["push", "-q"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git remote add` in the repository.
    /// 
    /// This adds a single remote to the repository. To call `git remote`
    /// with different options use [`RemoteOptions`].
    /// 
    /// [`RemoteOptions`]: commands/struct.RemoteOptions.html
    pub fn remote(&mut self, name: &str, url: &str) -> Result<&mut Self> {
        let args = vec!["remote", "add", name, url];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git status` parsing the status into idiomatic Rust type.
    /// 
    /// The status information is returned in a [`Status`].
    /// 
    /// [`Status`]: struct.Status.html
    pub fn status(&self) -> Result<Status> {
        let args = vec!["status", "--porcelain=v2", "--branch", "--ignored"];
        let out = self.run(args)?;
        Status::try_from(out.as_str())
    }

    /// Run `git stash` in the repository.
    /// 
    /// The command is run without ony options.
    pub fn stash(&mut self) -> Result<&mut Self> {
        let args = vec!["stash", "-q"];
        self.run(args)?;
        Ok(self)
    }

    /// Run `git tag`, creating a new tag object.
    /// 
    /// To call `git tag` with different options use [`TagOptions`].
    /// 
    /// [`TagOptions`]: commands/struct.TagOptions.html
    pub fn tag(&mut self, tagname: &str) -> Result<&mut Self> {
        let args = vec!["tag", tagname];
        self.run(args)?;
        Ok(self)
    }

    fn run(&self, args: Vec<&str>) -> Result<String> {
        // Setup command
        let mut cmd = Command::new("git");
        cmd.stdin(Stdio::inherit());
        if matches!(self.stdout, GitOut::Print) {
            cmd.stdout(Stdio::inherit())
               .stderr(Stdio::inherit());
        }
        if let Some(path) = &self.location {
            (&mut cmd).current_dir(path);
        }
        (&mut cmd).args(&args);

        if matches!(self.stdout, GitOut::Print) {
            // Run with inherited stdin/out
            let status = cmd.status().context("git execution failed")?;
            if status.success() {
                return Ok(String::new())
            } else {
                Err(GitError {
                    cmd: format!("git {}", args[0]),
                    code: status.code(),
                    stderr: format_err!("check stderr output"),
                }.into())
            }
        } else {
            // Run with piped stdin/out
            let out = cmd.output().context("git execution failed")?;
            if out.status.success() {
                Ok(String::from_utf8(out.stdout)?)
            } else {
                Err(GitError {
                    cmd: format!("git {}", args[0]),
                    code: out.status.code(),
                    stderr: format_err!("{}", std::str::from_utf8(&out.stderr)?),
                }.into())
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mhgit_unit() {
        assert_eq!(2 + 2, 4);
    }
}
