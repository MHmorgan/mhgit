//! Status types returned from git status.

// Copyright 2020 Magnus Aa. Hirth. All rights reserved.

use crate::GitError;
use failure::{Error, ResultExt};
use itertools::Itertools;
use std::convert::TryFrom;

/// Git status data.
///
/// ```rust,no_run
/// use mhgit::Repository;
///
/// fn main() {
///     let status = Repository::new().status().unwrap();
/// }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Status {
    /// Current local branch
    ///  0: commit, oid
    ///  1: head
    branch: (String, String),

    /// Upstream branch
    ///  0: name
    ///  1: behind current
    ///  2: ahead of current
    upstream: (String, u32, u32),

    /// Changed entries
    pub changed: Vec<Entry>,

    /// Renamed/copied entries
    pub renamed: Vec<Entry>,

    /// Untracked filenames
    pub untracked: Vec<String>,

    /// Ignored filenames
    pub ignored: Vec<String>,
}

/// A single entry from git status output.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Entry {
    // Leading format identifier
    pub format: char,

    // Status code (index, work tree)
    //  . = unmodified
    //  M = modified
    //  A = added
    //  D = deleted
    //  R = renamed
    //  C = copied
    //  U = updated but merged
    //
    //  0: index
    //  1: work tree
    status: (char, char),

    // Submodule state
    //  0: is submodule
    //  1: commit changed
    //  2: has tracked changes
    //  3: has untracked changes
    sub: (bool, bool, bool, bool),

    // Octal file mode
    //  0: HEAD
    //  1: index
    //  2: worktree
    file_mode: ([char; 6], [char; 6], [char; 6]),

    // Object name (oid)
    //  0: HEAD
    //  1: index
    object_name: (String, String),

    // The pathname. In a renamed/copied entry, this is the target path.
    path: String,

    // Rename/copy score (only renamed/copied entries)
    //  0: R - rename, C - copy
    //  1: similarity percentage
    score: (char, u8),

    // The pathname in the commit at HEAD or in the index (only renamed/copied entries)
    orig_path: String,

    /// Unmerged entry stages (only unmerged entries)
    ///  0: object name
    ///  1: file mode
    pub stage1: (String, [char; 6]),
    pub stage2: (String, [char; 6]),
    pub stage3: (String, [char; 6]),
}

impl Status {
    /// Return an empty status.
    #[inline]
    pub fn new() -> Status {
        Status { ..Default::default() }
    }

    /// Object id (oid) of current commit.
    #[inline]
    pub fn branch_oid(&self) -> &str {
        &self.branch.0
    }

    /// Head of current branch.
    #[inline]
    pub fn branch_head(&self) -> &str {
        &self.branch.1
    }

    /// Upstream branch, if set
    pub fn upstream_branch(&self) -> Option<&str> {
        if !self.upstream.0.is_empty() {
            Some(&self.upstream.0)
        } else {
            None
        }
    }

    /// Number of commits upstream is behind.
    pub fn upstream_behind(&self) -> Option<u32> {
        if !self.upstream.0.is_empty() {
            Some(self.upstream.1)
        } else {
            None
        }
    }

    /// Number of commits upstream is ahead.
    pub fn upstream_ahead(&self) -> Option<u32> {
        if !self.upstream.0.is_empty() {
            Some(self.upstream.2)
        } else {
            None
        }
    }
}

impl TryFrom<&str> for Status {
    type Error = Error;

    /// Parse captured output text from git status
    fn try_from(txt: &str) -> std::result::Result<Status, Self::Error> {
        macro_rules! err {
            () => {
                format_err!("bad status format")
            };
        };
        let mut status = Status { ..Default::default() };

        // Parse entries line by line
        for line in txt.lines() {
            let mut chars = line.chars();
            match chars.next() {
                // Branch info entry
                Some('#') => {
                    let _ = chars.next().ok_or(err!())?;
                    let info: String = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                    match info.as_str() {
                        "branch.oid" => status.branch.0 = chars.take_while(|c| !c.is_whitespace()).collect(),
                        "branch.head" => status.branch.1 = chars.take_while(|c| !c.is_whitespace()).collect(),
                        "branch.upstream" => status.upstream.0 = chars.take_while(|c| !c.is_whitespace()).collect(),
                        "branch.ab" => {
                            // Branch ahead
                            ensure!(Some('+') == chars.next(), err!());
                            let tmp: String = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                            status.upstream.1 = tmp.parse::<u32>().context(err!())?;
                            // Branch behind
                            ensure!(Some('-') == chars.next(), err!());
                            let tmp: String = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                            status.upstream.2 = tmp.parse::<u32>().context(err!())?;
                        },
                        _ => bail!("unknown branch info {}", line),
                    }
                }
                // Changed entry
                Some('1') => {
                    status.changed.push(Entry::try_from(line)?)
                }
                // Renamed/copied entry
                Some('2') => {
                    status.renamed.push(Entry::try_from(line)?)
                }
                // Untracked entry
                Some('?') => {
                    let _ = chars.next().ok_or(err!())?;
                    let path: String = chars.take_while(|c| !c.is_whitespace()).collect();
                    status.untracked.push(path);
                }
                // Ignored entry
                Some('!') => {
                    let _ = chars.next().ok_or(err!())?;
                    let path: String = chars.take_while(|c| !c.is_whitespace()).collect();
                    status.ignored.push(path);
                }
                // Unknown line prefix
                Some(_) => bail!("unknown line prefix in: {}", line),
                // Ignore empty lines
                None => (),
            }
        }

        Ok(status)
    }
}

impl Entry {
    /// Return an empty entry.
    pub fn new() -> Entry {
        Entry {
            ..Default::default()
        }
    }

    /// Returns true if it's a changed entry.
    #[inline]
    pub fn is_changed(&self) -> bool {
        self.format == '1'
    }

    /// Returns true if it's a remaned entry.
    #[inline]
    pub fn is_renamed(&self) -> bool {
        self.format == '2' && self.score.0 == 'R'
    }

    /// Return true if it's a copied entry.
    #[inline]
    pub fn is_copied(&self) -> bool {
        self.format == '2' && self.score.0 == 'C'
    }

    /// Returns true if it's an unmerged entry.
    #[inline]
    pub fn is_unmerged(&self) -> bool {
        self.format == 'u'
    }

    /// Returns true if it's an untracked entry.
    #[inline]
    pub fn is_untracked(&self) -> bool {
        self.format == '?'
    }

    /// Return true if it's an ignored entry.
    #[inline]
    pub fn is_ignored(&self) -> bool {
        self.format == '!'
    }

    /// Return modified state of the index and work tree, respectively.
    /// 
    /// * `.` : unmodified
    /// * `M` : modified
    /// * `A` : added
    /// * `D` : deleted
    /// * `R` : renamed
    /// * `C` : copied
    /// * `U` : updated but merged
    /// 
    #[inline]
    pub fn modified_state(&self) -> (char, char) {
        self.status
    }

    /// Return true if entry is a submodule
    #[inline]
    pub fn is_submodule(&self) -> bool {
        self.sub.0
    }

    /// Return true if the submodule commit changed.
    #[inline]
    pub fn sub_commit_changed(&self) -> bool {
        self.sub.1
    }

    /// Return true if the submodule has tracked changes
    #[inline]
    pub fn sub_tracked_changes(&self) -> bool {
        self.sub.2
    }

    /// Return true if the submodule has untracked changes
    #[inline]
    pub fn sub_untracked_changes(&self) -> bool {
        self.sub.3
    }

    /// Six character octial file mode in HEAD. 
    #[inline]
    pub fn file_mode_head(&self) -> &[char] {
        &self.file_mode.0
    }

    /// Six character octial file mode in the index. 
    #[inline]
    pub fn file_mode_index(&self) -> &[char] {
        &self.file_mode.1
    }

    /// Six character octial file mode in the worktree. 
    #[inline]
    pub fn file_mode_worktree(&self) -> &[char] {
        &self.file_mode.2
    }

    /// Object name (oid) in HEAD.
    #[inline]
    pub fn object_name_head(&self) -> &str {
        &self.object_name.0
    }
    
    /// Object name (oid) in the index.
    #[inline]
    pub fn object_name_index(&self) -> &str {
        &self.object_name.1
    }

    /// Pathname. In a renamed/copied entry, this is the target path.
    #[inline]
    pub fn pathname(&self) -> &str {
        &self.path
    }

    /// Return the score denoting the percentage of similarity between the
    /// source and target of the move or copy. 
    /// 
    /// If the entry is not renamed/copied this value should be ignored.
    #[inline]
    pub fn score(&self) -> u8 {
        self.score.1
    }

    /// The pathname in the commit at HEAD or in the index.
    /// 
    /// If the entry is not renamed/copied this value should be ignored.
    #[inline]
    pub fn orig_path(&self) -> &str {
        &self.orig_path
    }
}

impl TryFrom<&str> for Entry {
    type Error = Error;

    /// Parse an entry line as printed by git.
    fn try_from(txt: &str) -> std::result::Result<Entry, Self::Error> {
        macro_rules! err {
            () => {
                format_err!("bad entry format")
            };
        };
        let mut chars = txt.chars();
        let mut entry = Entry {
            ..Default::default()
        };

        // First character should be format identifier
        entry.format = chars.next().ok_or(err!())?;
        let _ = chars.next().ok_or(err!())?; // space

        match entry.format {
            // Changed entry
            '1' => {
                // <XY>
                entry.status = (&mut chars).take(2).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?;
                // <sub>
                let sub: (_, _, _, _) = (&mut chars).take(4).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?; // space
                entry.sub = (sub.0 == 'S', sub.1 == 'C', sub.2 == 'M', sub.3 == 'U');
                // <mH>
                let fm_head: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.0.copy_from_slice(fm_head.as_slice());
                // <mI>
                let fm_index: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.1.copy_from_slice(fm_index.as_slice());
                // <mW>
                let fm_worktree: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.2.copy_from_slice(fm_worktree.as_slice());
                // <hH>
                entry.object_name.0 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <hI>
                entry.object_name.1 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <path>
                entry.path = chars.collect();
            }

            // Renamed/modified entry
            '2' => {
                // <XY>
                entry.status = (&mut chars).take(2).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?;
                // <sub>
                let sub: (_, _, _, _) = (&mut chars).take(4).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?; // space
                entry.sub = (sub.0 == 'S', sub.1 == 'C', sub.2 == 'M', sub.3 == 'U');
                // <mH>
                let fm_head: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.0.copy_from_slice(fm_head.as_slice());
                // <mI>
                let fm_index: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.1.copy_from_slice(fm_index.as_slice());
                // <mW>
                let fm_worktree: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.2.copy_from_slice(fm_worktree.as_slice());
                // <hH>
                entry.object_name.0 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <hI>
                entry.object_name.1 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <X><score>
                entry.score.0 = (&mut chars).next().ok_or(err!())?;
                let tmp: String = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                entry.score.1 = tmp.parse::<u8>().context(err!())?;
                // <path>
                entry.path = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <origPath>
                entry.orig_path = (&mut chars).collect();
            }

            // Unmerged entry
            'u' => {
                // <XY>
                entry.status = (&mut chars).take(2).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?;
                // <sub>
                let sub: (_, _, _, _) = (&mut chars).take(4).collect_tuple().ok_or(err!())?;
                let _ = chars.next().ok_or(err!())?; // space
                entry.sub = (sub.0 == 'S', sub.1 == 'C', sub.2 == 'M', sub.3 == 'U');
                // <m1>
                let fm_stage1: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.stage1.1.copy_from_slice(fm_stage1.as_slice());
                // <m2>
                let fm_stage2: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.stage2.1.copy_from_slice(fm_stage2.as_slice());
                // <m3>
                let fm_stage3: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.stage3.1.copy_from_slice(fm_stage3.as_slice());
                // <mW>
                let fm_worktree: Vec<_> = (&mut chars).take(6).collect();
                let _ = chars.next().ok_or(err!())?; // space
                entry.file_mode.2.copy_from_slice(fm_worktree.as_slice());
                // <h1>
                entry.stage1.0 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <h2>
                entry.stage2.0 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <h3>
                entry.stage3.0 = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
                // <path>
                entry.path = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
            }

            // Untracked entry
            '?' => {
                // <path>
                entry.path = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
            }

            // Ignored entry
            '!' => {
                // <path>
                entry.path = (&mut chars).take_while(|c| !c.is_whitespace()).collect();
            }

            _ => bail!("unknown entry format identifier (should be one of: 1 2 u ? !)"),
        };
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static OUT: &'static str = "# branch.oid dbcbc3608451f09fffef8f31a2a54da54aa13a87
# branch.head master
# branch.upstream origin/master
# branch.ab +1 -0
1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 e47c0835424019d3cb9f3daf768eafbb2fd42044 Cargo.toml
1 .M N... 100644 100644 100644 567578ae6981902a62d42f69599a1101e33a0bba 567578ae6981902a62d42f69599a1101e33a0bba README.md
? LICENSE~
? Makefile
";

    #[test]
    fn entry() {
        let entry = Entry::new();
        assert!(
            entry == Entry::default(),
            "Entry::new() doesn't return a default entry"
        );
    }

    #[test]
    fn entry_parsing() {
        // Changed 1

        let changed1 = "1 A. N... 000000 100644 100644 0000000000000000000000000000000000000000 df6d704ad8308efda4715321c69c9aff1fc95e0e TODO.md";
        let ch_want1 = Entry {
            format: '1',
            status: ('A', '.'),
            sub: (false, false, false, false),
            file_mode: (
                ['0', '0', '0', '0', '0', '0'],
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
            ),
            object_name: (
                String::from("0000000000000000000000000000000000000000"),
                String::from("df6d704ad8308efda4715321c69c9aff1fc95e0e"),
            ),
            path: String::from("TODO.md"),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(changed1).expect("failed to parse changed1 entry"), ch_want1, "Changed entry 1 not parsed correctly");

        // Changed 2

        let changed2 = "1 MD SCMU 000000 100644 100644 0000000000000000000000000000000000000000 df6d704ad8308efda4715321c69c9aff1fc95e0e TODO.md";
        let ch_want2 = Entry {
            format: '1',
            status: ('M', 'D'),
            sub: (true, true, true, true),
            file_mode: (
                ['0', '0', '0', '0', '0', '0'],
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
            ),
            object_name: (
                String::from("0000000000000000000000000000000000000000"),
                String::from("df6d704ad8308efda4715321c69c9aff1fc95e0e"),
            ),
            path: String::from("TODO.md"),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(changed2).expect("failed to parse changed2 entry"), ch_want2, "Changed entry 2 not parsed correctly");

        // Renamed

        let renamed = "2 R. N... 100644 100644 100644 288d723fce8678bcdcb40bfa844a6f815d625661 288d723fce8678bcdcb40bfa844a6f815d625661 R100 LICENSE	LICENSE~";
        let rn_want = Entry {
            format: '2',
            status: ('R', '.'),
            sub: (false, false, false, false),
            file_mode: (
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
            ),
            object_name: (
                String::from("288d723fce8678bcdcb40bfa844a6f815d625661"),
                String::from("288d723fce8678bcdcb40bfa844a6f815d625661"),
            ),
            path: String::from("LICENSE"),
            score: ('R', 100),
            orig_path: String::from("LICENSE~"),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(renamed).expect("failed to parse renamed entry"), rn_want, "Renamed entry not parsed correctly");

        // Unmerged

        let unmerged = "u MM N... 000000 100644 100644 100755 0000000000000000000000000000000000000000 288d723fce8678bcdcb40bfa844a6f815d625661 288d723fce8678bcdcb40bfa844a6f815d625661 LICENSE";
        let um_want2 = Entry {
            format: 'u',
            status: ('M', 'M'),
            sub: (false, false, false, false),
            file_mode: (['\u{0}'; 6], ['\u{0}'; 6], ['1', '0', '0', '7', '5', '5']),
            path: String::from("LICENSE"),
            stage1: (
                String::from("0000000000000000000000000000000000000000"),
                ['0', '0', '0', '0', '0', '0'],
            ),
            stage2: (
                String::from("288d723fce8678bcdcb40bfa844a6f815d625661"),
                ['1', '0', '0', '6', '4', '4'],
            ),
            stage3: (
                String::from("288d723fce8678bcdcb40bfa844a6f815d625661"),
                ['1', '0', '0', '6', '4', '4'],
            ),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(unmerged).expect("failed to parse unmerged entry"), um_want2, "Unmerged entry not parsed correctly");

        // Untracked

        let untracked = "? ufile.txt";
        let ut_want = Entry {
            format: '?',
            path: String::from("ufile.txt"),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(untracked).expect("failed to parse untracked entry"), ut_want, "Untracked entry not parsed correctly");

        // Ignored

        let ignored = "! idir/";
        let ig_want = Entry {
            format: '!',
            path: String::from("idir/"),
            ..Default::default()
        };
        assert_eq!(Entry::try_from(ignored).expect("failed to parse ignored entry"), ig_want, "Ignored entry not parsed correctly");
    }

    #[test]
    #[should_panic(expected = "unknown entry format identifier (should be one of: 1 2 u ? !)")]
    fn invalid_entry_parsing() {
        Entry::try_from("0 Foo/Bar.txt").unwrap();
    }

    #[test]
    fn status() {
        assert_eq!(Status::new(), Status { ..Default::default() }, "Status::new is not default");

        let status = Status::try_from(OUT).expect("failed to create status from output text");
        let entry1 = Entry {
            format: '1',
            status: ('A', '.'),
            sub: (false, false, false, false),
            file_mode: (
                ['0', '0', '0', '0', '0', '0'],
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
            ),
            object_name: (
                String::from("0000000000000000000000000000000000000000"),
                String::from("e47c0835424019d3cb9f3daf768eafbb2fd42044"),
            ),
            path: String::from("Cargo.toml"),
            ..Default::default()
        };
        let entry2 = Entry {
            format: '1',
            status: ('.', 'M'),
            sub: (false, false, false, false),
            file_mode: (
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
                ['1', '0', '0', '6', '4', '4'],
            ),
            object_name: (
                String::from("567578ae6981902a62d42f69599a1101e33a0bba"),
                String::from("567578ae6981902a62d42f69599a1101e33a0bba"),
            ),
            path: String::from("README.md"),
            ..Default::default()
        };
        let want = Status {
            branch: (String::from("dbcbc3608451f09fffef8f31a2a54da54aa13a87"), String::from("master")),
            upstream: (String::from("origin/master"), 1, 0),
            changed: vec![entry1, entry2],
            renamed: Vec::new(),
            untracked: vec!["LICENSE~".to_string(), "Makefile".to_string()],
            ignored: Vec::new(),
        };
        assert_eq!(status, want, "Status not parsed correctly");

        // Methods
        assert_eq!(status.branch_oid(), "dbcbc3608451f09fffef8f31a2a54da54aa13a87");
        assert_eq!(status.branch_head(), "master");
        assert_eq!(status.upstream_branch(), Some("origin/master"));
        assert_eq!(status.upstream_behind(), Some(1));
        assert_eq!(status.upstream_ahead(), Some(0));
    }
}
