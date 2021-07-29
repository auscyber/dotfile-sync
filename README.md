
# Dotfile Sync 
###### For UNIX-based systems

![Build](https://img.shields.io/github/workflow/status/auscyberman/dotfile-sync/tests)
![Issues](https://img.shields.io/github/issues/auscyberman/dotfile-sync?color=pink)


**Syncing dotfiles or other folders with symlinks can be a bit annoying to manage. Especially when you have multiple systems to setup.**

You can create system dependent links to alleviate all your stress

## Table of Contents

1. [Features](#features)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Usage](#usage)


## Features <a name="features"></a>

* Easy initalisation of configurations  
    `dots init`
* Addition of links  
    `dots add file1 file2`  
    `dots add file1 --destination files/file2linked`  
    `dots add file1 file2 --destination files`
* Manage globally  
    `dots manage`
    `dots manage --default`
* Only link on specific system  
    `dots --system laptop add file1laptop`  
    `dots --system desktop add file1desktop file2desktop --destination files`
* Read from environment variables  
    `dots add $HOME/file1`
* Project-wide variables
    ```toml
    [variables]
    user = "auscyber"
    password = "1234"
    ```
    ![List example](https://i.imgur.com/EMem4sN.png)
* Revert link  
    `dots revert file1`


## Installation <a name="installation"></a>

1. Install rust-toolchain with if you don't already have it installed  
    `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` 
3. Clone the repository into a folder of your choice  
    `git clone https://github.com/auscyberman/dotfile-sync`
4. Run `cargo build --release` and add `target/build/release/dots` to your `PATH`

**Now It's installed. Time to configure it**

## Configuration <a name="configuration"></a>

### Create base `.links.toml` file
1. Move to folder where you want to initalise project
2. Initalise Project  `dots init` 
3. `dots manage --default` *Note: `default` flag not required, just simplifies later tasks*  

**Now you are all setup, just run `dots add file1 file2 ... ` and `dots sync` to sync everything on another computer**
*Note: adding files requires `--project` or `--project-path` flag to command if `--default` wasn't added to `manage`*

**Dotfile Sync uses [TOML](https://github.com/toml-lang/toml) for configuration**

* To add a link to the project, add a new section
```toml
[[links]]
```
with these attributes
* `name`: The user side name for the link
* `src`:  The relative location of the actual file in the project
* `destination`: The location for the `src` to be linked to

## Usage <a name="usage"></a>
#### Adding multiple files
To add the files `file1` `file2`

`dots add file1 file2`

#### Set destination of files
To put the file `file1` in project/`files/file2`

`dots add file1 -d files/file2`

#### Add file dependent on system
To only link `file` when the system is `desktop`  
`dots add file1 --system "desktop"`

#### On another computer
To sync with no extra paremeters  
`curl https://git.io/JBB45 | sh -s `  
To add extra paremeters `--system "desktop"`  
`curl https://git.io/JBB45 | sh -s -- --system "desktop"`

