//! Git command types: `AddOptions`, `PushOptions`, etc.

use crate::{CommandOptions, GitError, Repository, Result};
use failure::ResultExt;
use std::process::{self, Command, Output, Stdio};

/// `git add` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{Repository, CommandOptions};
/// use mhgit::commands::AddOptions;
///
/// let repo = Repository::new();
/// AddOptions::new()
///            .chmod(false)
///            .all(true)
///            .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct AddOptions {
    all: Option<bool>,
    chmod: Option<bool>,
    pathspecs: Vec<String>,
}

impl AddOptions {
    /// Create a new set of `git add` options.
    pub fn new() -> AddOptions {
        AddOptions {
            ..Default::default()
        }
    }

    /// Add argument:  
    /// * `true` : --all
    /// * `false` : --no-all
    pub fn all(&mut self, val: bool) -> &mut AddOptions {
        self.all = Some(val);
        self
    }

    /// Add argument:
    /// * `true` : --chmod=+x
    /// * `false` : --chmod=-x
    pub fn chmod(&mut self, val: bool) -> &mut AddOptions {
        self.chmod = Some(val);
        self
    }

    /// Add a pathspec to add command.
    pub fn pathspec(&mut self, pathspec: impl ToString) -> &mut AddOptions {
        self.pathspecs.push(pathspec.to_string());
        self
    }

    /// Add multiple pathspecs to add command.
    pub fn pathspecs<I, S>(&mut self, pathspecs: I) -> &mut AddOptions
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        for p in pathspecs {
            self.pathspecs.push(p.to_string());
        }
        self
    }
}

impl CommandOptions for AddOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["add"];
        // add
        if let Some(all) = self.all {
            if all {
                args.push("--all");
            } else {
                args.push("--no-all");
            }
        }
        // chmode
        if let Some(chmod) = self.chmod {
            if chmod {
                args.push("--chmod=+x");
            } else {
                args.push("--chmod=-x");
            }
        }
        // pathspec
        for p in &self.pathspecs {
            args.push(&p);
        }
        args
    }

    #[inline]
    fn parse_output(&self, _out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git clone` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::commands::CloneOptions;
/// CloneOptions::new()
///     .origin("upstream")
///     .branch("dev")
///     .run("https://repo.com/foobar.git")?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct CloneOptions {
    branch: Option<String>,
    origin: Option<String>,
    url: Option<String>,
    dir: Option<String>,
}

impl CloneOptions {
    /// Create a new set of `git clone` options.
    pub fn new() -> CloneOptions {
        CloneOptions {
            ..Default::default()
        }
    }

    /// Point to given branch in cloned repository instead of HEAD.
    pub fn branch(&mut self, name: &str) -> &mut Self {
        self.branch = Some(name.to_string());
        self
    }

    /// Instead of using the remote name origin to keep track of the upstream repository, use <name>.
    pub fn origin(&mut self, name: &str) -> &mut Self {
        self.origin = Some(name.to_string());
        self
    }

    /// Specify to cline into.
    pub fn dir(&mut self, dir: &str) -> &mut Self {
        self.dir = Some(dir.to_string());
        self
    }

    /// Clone the repository. `repository` is the repo URL.
    pub fn run(&self, repository: &str) -> Result<Repository> {
        // Setup git arguments
        let mut args = vec!["clone"];
        if let Some(branch) = &self.branch {
            args.push("--branch");
            args.push(branch.as_str());
        }
        if let Some(origin) = &self.origin {
            args.push("--origin");
            args.push(origin.as_str());
        }
        args.push(repository);
        if let Some(dir) = &self.dir {
            args.push(dir.as_str());
        }

        // Run command
        let mut cmd = Command::new("git");
        (&mut cmd).args(&args);
        let out = cmd.output().context("git execution failed")?;

        if out.status.success() {
            if let Some(dir) = &self.dir {
                Ok(Repository::at(dir)?)
            } else {
                Ok(Repository::new())
            }
        } else {
            Err(GitError {
                cmd: "git clone".to_string(),
                code: out.status.code(),
                stderr: format_err!("{}", std::str::from_utf8(&out.stderr)?),
            }
            .into())
        }
    }
}

/// `git commit` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::CommitOptions;
///
/// let repo = Repository::new();
/// CommitOptions::new()
///     .amend(true)
///     .file("foo.txt")
///     .message("Initial commit")
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct CommitOptions {
    all: bool,
    allow_empty: bool,
    amend: bool,
    files: Vec<String>,
    msg: String,
}

impl CommitOptions {
    /// Create a new set of `git commit` options.
    pub fn new() -> CommitOptions {
        CommitOptions {
            ..Default::default()
        }
    }

    /// Add --all options.
    pub fn all(&mut self, val: bool) -> &mut CommitOptions {
        self.all = val;
        self
    }

    /// Add --allow-empty option.
    pub fn allow_empty(&mut self, val: bool) -> &mut CommitOptions {
        self.allow_empty = val;
        self
    }

    /// Add --amend option.
    pub fn amend(&mut self, val: bool) -> &mut CommitOptions {
        self.amend = val;
        self
    }

    /// Add file to commit command.
    pub fn file(&mut self, file: impl ToString) -> &mut CommitOptions {
        self.files.push(file.to_string());
        self
    }

    /// Add multiple files to commit command.
    pub fn files<I, S>(&mut self, files: I) -> &mut CommitOptions
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        for f in files {
            self.files.push(f.to_string());
        }
        self
    }

    /// Set commit message.
    pub fn message(&mut self, msg: &str) -> &mut CommitOptions {
        self.msg = msg.to_owned();
        self
    }
}

impl CommandOptions for CommitOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["commit", "-q"];
        if !self.msg.is_empty() {
            args.push("-m");
            args.push(&self.msg);
        }
        if self.all {
            args.push("--all");
        }
        if self.allow_empty {
            args.push("--allow-empty");
        }
        if self.amend {
            args.push("--amend");
        }
        for file in &self.files {
            args.push(&file);
        }
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git notes` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::NotesOptions;
///
/// let repo = Repository::new();
/// NotesOptions::add()
///     .message("My note")
///     .object("HEAD")
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct NotesOptions {
    action: String,
    msg: String,
    object: String,
}

impl NotesOptions {
    /// Create a new set of `git notes add` options.
    pub fn add() -> NotesOptions {
        NotesOptions {
            action: "add".to_owned(),
            ..Default::default()
        }
    }

    /// Create a new set of `git notes append` options.
    pub fn append() -> NotesOptions {
        NotesOptions {
            action: "append".to_owned(),
            ..Default::default()
        }
    }

    /// Create a new set of `git notes remove` options.
    pub fn remove() -> NotesOptions {
        NotesOptions {
            action: "remove".to_owned(),
            ..Default::default()
        }
    }

    /// Set commit message.
    pub fn message(&mut self, msg: &str) -> &mut NotesOptions {
        self.msg = msg.to_owned();
        self
    }

    /// Set object to attach notes to.
    pub fn object(&mut self, object: &str) -> &mut NotesOptions {
        self.object = object.to_owned();
        self
    }
}

impl CommandOptions for NotesOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["notes", &self.action];
        if !self.msg.is_empty() {
            args.push("-m");
            args.push(&self.msg);
        }
        if !self.object.is_empty() {
            args.push(&self.object);
        }
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git pull` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::PullOptions;
///
/// let repo = Repository::new();
/// PullOptions::new()
///     .remote("origin")
///     .refspec("master")
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct PullOptions {
    allow_unrelated: bool,
    repository: String,
    refspecs: Vec<String>,
}

impl PullOptions {
    /// Create a new set of `git pull` options.
    pub fn new() -> PullOptions {
        PullOptions {
            ..Default::default()
        }
    }

    /// Add --allow-unrelated option.
    pub fn allow_unrelated(&mut self, val: bool) -> &mut PullOptions {
        self.allow_unrelated = val;
        self
    }

    /// Add refspec to pull command.
    pub fn refspec(&mut self, file: impl ToString) -> &mut PullOptions {
        self.refspecs.push(file.to_string());
        self
    }

    /// Add multiple refspecs to pull command.
    pub fn refspecs<I, S>(&mut self, files: I) -> &mut PullOptions
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        for f in files {
            self.refspecs.push(f.to_string());
        }
        self
    }

    /// Set remote repository source.
    pub fn remote(&mut self, repo: impl ToString) -> &mut PullOptions {
        self.repository = repo.to_string();
        self
    }
}

impl CommandOptions for PullOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["pull", "-q"];
        if self.allow_unrelated {
            args.push("--allow-unrelated");
        }
        if !self.repository.is_empty() {
            args.push(&self.repository);
        }
        for rs in &self.refspecs {
            args.push(&rs);
        }
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git push` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::PushOptions;
///
/// let repo = Repository::new();
/// PushOptions::new()
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct PushOptions {
    all: bool,
    tags: bool,
    force: bool,
    set_upstream: bool,
    repository: String,
    refspecs: Vec<String>,
}

impl PushOptions {
    /// Create a new set of `git tag` options.
    pub fn new() -> PushOptions {
        PushOptions {
            ..Default::default()
        }
    }

    /// Add --all option.
    pub fn all(&mut self, val: bool) -> &mut PushOptions {
        self.all = val;
        self
    }

    /// Add --tags option.
    pub fn tags(&mut self, val: bool) -> &mut PushOptions {
        self.tags = val;
        self
    }

    /// Add --force option.
    pub fn force(&mut self, val: bool) -> &mut PushOptions {
        self.force = val;
        self
    }

    /// Add --set-upstream option.
    pub fn set_upstream(&mut self, val: bool) -> &mut PushOptions {
        self.set_upstream = val;
        self
    }

    /// Add refspec to push command.
    pub fn refspec(&mut self, file: impl ToString) -> &mut PushOptions {
        self.refspecs.push(file.to_string());
        self
    }

    /// Add multiple refspecs to push command.
    pub fn refspecs<I, S>(&mut self, files: I) -> &mut PushOptions
    where
        I: IntoIterator<Item = S>,
        S: ToString,
    {
        for f in files {
            self.refspecs.push(f.to_string());
        }
        self
    }

    /// Set remote repository source.
    pub fn remote(&mut self, repo: impl ToString) -> &mut PushOptions {
        self.repository = repo.to_string();
        self
    }
}

impl CommandOptions for PushOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["push", "-q"];
        if self.all {
            args.push("--all");
        }
        if self.tags {
            args.push("--tags");
        }
        if self.force {
            args.push("--force");
        }
        if self.set_upstream {
            args.push("--set-upstream");
        }
        if !self.repository.is_empty() {
            args.push(&self.repository);
        }
        for rs in &self.refspecs {
            args.push(&rs);
        }
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git remote` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::RemoteOptions;
///
/// let repo = Repository::new();
/// RemoteOptions::add()
///     .master("master")
///     .name("upstream")
///     .url("https://web.com/myrepo.git")
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct RemoteOptions {
    action: String,
    master: String,
    tags: Option<bool>,
    name: String,
    url: String,
}

impl RemoteOptions {
    /// Create a new set of `git remote add` options.
    pub fn add() -> RemoteOptions {
        RemoteOptions {
            action: "add".to_string(),
            ..Default::default()
        }
    }

    /// Add -m <master> option.
    pub fn master(&mut self, name: &str) -> &mut RemoteOptions {
        self.master = name.to_string();
        self
    }

    /// Add --tags or --no-tags option.
    pub fn tags(&mut self, val: bool) -> &mut RemoteOptions {
        self.tags = Some(val);
        self
    }

    /// Set <name> parameter.
    pub fn name(&mut self, name: &str) -> &mut RemoteOptions {
        self.name = name.to_string();
        self
    }

    /// Set <url> parameter.
    pub fn url(&mut self, url: &str) -> &mut RemoteOptions {
        self.url = url.to_string();
        self
    }
}

impl CommandOptions for RemoteOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["remote", &self.action];
        if !self.master.is_empty() {
            args.push("-m");
            args.push(&self.master);
        }
        match &self.tags {
            Some(true) => args.push("--tags"),
            Some(false) => args.push("--no-tags"),
            None => (),
        }
        args.push(&self.name);
        args.push(&self.url);
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/// `git tag` command.
///
/// ```rust,no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use mhgit::{CommandOptions, Repository};
/// use mhgit::commands::TagOptions;
///
/// let repo = Repository::new();
/// TagOptions::add()
///     .msg("A new tag")
///     .tagname("v0.0")
///     .run(&repo)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct TagOptions {
    action: String,
    msg: String,
    tagname: String,
    // commit/object
    object: String,
}

impl TagOptions {
    /// Create a new set of `git tag` options.
    pub fn add() -> TagOptions {
        TagOptions {
            action: "add".to_string(),
            ..Default::default()
        }
    }

    /// Create a new set of `git tag -d` options.
    pub fn delete() -> TagOptions {
        TagOptions {
            action: "delete".to_string(),
            ..Default::default()
        }
    }

    /// Set tag message.
    pub fn msg(&mut self, msg: &str) -> &mut TagOptions {
        self.msg = msg.to_string();
        self
    }

    /// Set tagname.
    pub fn tagname(&mut self, name: &str) -> &mut TagOptions {
        self.tagname = name.to_string();
        self
    }

    /// Set commit the tag will refer to.
    pub fn commit(&mut self, commit: &str) -> &mut TagOptions {
        self.object = commit.to_string();
        self
    }

    /// Set object the tag will refer to.
    pub fn object(&mut self, object: &str) -> &mut TagOptions {
        self.object = object.to_string();
        self
    }
}

impl CommandOptions for TagOptions {
    type Output = ();

    fn git_args(&self) -> Vec<&str> {
        let mut args = vec!["tag"];
        if self.action == "delete" {
            args.push("-d");
        }
        if !self.msg.is_empty() {
            args.push("-m");
            args.push(&self.msg);
        }
        args.push(&self.tagname);
        if !self.object.is_empty() {
            args.push(&self.object);
        }
        args
    }

    #[inline]
    fn parse_output(&self, out: &str) -> Result<Self::Output> {
        Ok(())
    }
}

/*******************************************************************************
 *                                                                             *
 * Test
 *                                                                             *
 *******************************************************************************/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        assert_eq!(AddOptions::new().git_args(), vec!["add"]);
        assert_eq!(
            AddOptions::new()
                .all(false)
                .chmod(false)
                .pathspec("foobar")
                .git_args(),
            vec!["add", "--no-all", "--chmod=-x", "foobar"]
        );
        assert_eq!(
            AddOptions::new()
                .all(true)
                .chmod(true)
                .pathspecs(&["foo", "bar"])
                .git_args(),
            vec!["add", "--all", "--chmod=+x", "foo", "bar"]
        );
    }

    #[test]
    fn commit() {
        assert_eq!(CommitOptions::new().git_args(), vec!["commit", "-q"]);
        assert_eq!(
            CommitOptions::new()
                .message("tull")
                .all(true)
                .allow_empty(true)
                .amend(true)
                .file("Makefile")
                .files(vec!["foo.txt", "bar.txt"])
                .git_args(),
            vec![
                "commit",
                "-q",
                "-m",
                "tull",
                "--all",
                "--allow-empty",
                "--amend",
                "Makefile",
                "foo.txt",
                "bar.txt"
            ]
        );
    }

    #[test]
    fn notes() {
        assert_eq!(
            NotesOptions::add()
                .message("test")
                .object("HEAD")
                .git_args(),
            vec!["notes", "add", "-m", "test", "HEAD"]
        );
        assert_eq!(
            NotesOptions::append()
                .message("test")
                .object("HEAD")
                .git_args(),
            vec!["notes", "append", "-m", "test", "HEAD"]
        );
        assert_eq!(
            NotesOptions::remove().object("HEAD").git_args(),
            vec!["notes", "remove", "HEAD"]
        );
    }

    #[test]
    fn pull() {
        assert_eq!(PullOptions::new().git_args(), vec!["pull", "-q"]);
        assert_eq!(
            PullOptions::new()
                .allow_unrelated(true)
                .remote("origin")
                .refspec("master")
                .git_args(),
            vec!["pull", "-q", "--allow-unrelated", "origin", "master"]
        );
    }

    #[test]
    fn push() {
        assert_eq!(PushOptions::new().git_args(), vec!["push", "-q"]);
        assert_eq!(
            PushOptions::new()
                .all(true)
                .tags(true)
                .force(true)
                .set_upstream(true)
                .remote("origin")
                .refspec("master")
                .git_args(),
            vec![
                "push",
                "-q",
                "--all",
                "--tags",
                "--force",
                "--set-upstream",
                "origin",
                "master"
            ]
        );
    }

    #[test]
    fn remote() {
        assert_eq!(
            RemoteOptions::add()
                .master("master")
                .tags(true)
                .name("origin")
                .url("git://myrepo.com")
                .git_args(),
            vec![
                "remote",
                "add",
                "-m",
                "master",
                "--tags",
                "origin",
                "git://myrepo.com"
            ]
        );
    }

    #[test]
    fn tag() {
        assert_eq!(
            TagOptions::add()
                .msg("testen")
                .tagname("v1.0")
                .object("HEAD")
                .git_args(),
            vec!["tag", "-m", "testen", "v1.0", "HEAD"]
        );
        assert_eq!(
            TagOptions::delete().tagname("v1.0").git_args(),
            vec!["tag", "-d", "v1.0"]
        );
    }
}
