![MHgit](./mhgit.png)

[![Travis CI build status](https://img.shields.io/travis/com/MHmorgan/mhgit/master?style=flat-square)](https://travis-ci.com/MHmorgan/mhgit)
[![Crates.io latest version](https://img.shields.io/crates/v/mhgit?style=flat-square)](https://crates.io/crates/mhgit)
![Crates.io downloads](https://img.shields.io/crates/d/mhgit?style=flat-square)
![GitHub license](https://img.shields.io/github/license/MHmorgan/mhgit?style=flat-square)


MHgit is a simple git library for interracting with git repositories. Provides an idiomatic and easy
way of dealing with git repos.

Requires git to be installed on the system.

#### Supported actions

* `add`
* `clone`
* `commit`
* `init`
* `notes`
* `pull`
* `push`
* `remote`
* `status`
* `stash`
* `tag`

Example
-------

```run
extern crate mhgit;

use mhgit::{CommandOptions, Repository};
use mhgit::commands::{PushOptions, RemoteOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::at("/home/mh/awesomeness")?
        .init()?
        .add()?
        .commit("Initial commit")?;
    RemoteOptions::add()
        .master("master")
        .name("upstream")
        .url("https://web.com/myrepo.git")
        .run(&repo)?;
    PushOptions::new()
        .set_upstream(true)
        .remote("origin")
        .refspec("master")
        .run(&repo)?;
    Ok(())
}
```


Changelog
---------

