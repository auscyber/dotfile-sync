
# Dotfile Sync 

![Build](https://img.shields.io/github/workflow/status/auscyberman/dotfile-sync/tests)
![Issues](https://img.shields.io/github/issues/auscyberman/dotfile-sync?color=pink)


**Syncing dotfiles or other folders with symlinks can be a bit annoying to manage. Especially when you have multiple systems to setup.**

You can create system dependent links to alleviate all your stress



## Features

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
* Projectwide Variables
    ```toml
    [variables]
    user = "auscyber"
    password = "1234"
    ```
![List example](https://i.imgur.com/EMem4sN.png)
* Revert link  
    `dots revert file1`



## Setup

1. Move to folder where you want to initalise project
2. 

