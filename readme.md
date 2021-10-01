RuShell
======

A Shell built in rust build around the bogobble parser system.

It has a few features not many shells have:

* syntax and error highlighting as you type.
* multiline edits 
* raw string input 
* default values if var not found

There is still plenty to do, but many things work

install using "cargo install ru\_shell" 

To set environment up you create a file called "$HOME/.config/rushell/init.rush"

In it put something like 

A normal looking shell prompt

```text
let RU_PROMPT=r#"${USERNAME,rushell} \e33m$PWD\n>>\e0m"#
```
Here :

* ```r#".."#``` reads a raw string whose contents will not be processed on input. This allows it to be processed, everytime the prompt needs to show.
* ```\\e``` is equivilent ```Esc[``` Which is used to signal escape codes.
* ```33m``` / ```34m``` Sets the color
* ```0m``` resets the color to defaults.
* ```${varname, default}``` if the var is empty, will use the default value
* ```$PWD``` refers to the variable directly but will throw show an error if the variable does not exist.
* ```$(command args)``` runs the command, and the output is treated as a single string arg.


The prompt I normally use

```text
let RU_PROMPT= r#"\e1m\e34m${USERNAME,Matt}: :\e32m$(basename $PWD)\e34m>>\e0m"#
```

if you have [starship](https://starship.rs/) installed you can use it like this:

```text
let RU_PROMPT = r"$(starship prompt)"
```

The other environment variables RuShell explicitly uses are
"PWD", "PATH" ,"RU\_HIGHLIGHT", "RU\_COMPLETE", however the latter two are not completely settled yet.


The "init.rush" file will be run at the beginning of each shell, and it handles acts as though you had just typed them all in at the file at the beginning of the session.

## Usage

In General using Ru Shell should feel much like using any other shell, with a few notable exceptions.

Blocks are curly braced

```text
for x in * {
echo $x
}

if true {
echo Its True
} else {
echo Its False
}

```

There is a keyword for disown

```text
disown syncthing --no-browser
```

## Assigners : Export, Let, Set, Push

There are four ways to write to variables, they all look the same:

```text
let x = a
set x = b
push x = :c
export x = d
echo $x
# prints b:c 
```

Here's what they do

* "let" creates a variable in the current scope with the appropriate value
* "set" searches for a variable in the smallest scope it can with the given name and replaces it with the value. If none are found, creates one in the current scope.
* "push" searches for a variable and pushes the new value on the end. 
* "export" writes to an Environment Variable"

when reading variables, The current scope is checked first, then outwards, until finally environment variables.

All assigners can assign multiple variables at once.

```text
let x y = a [4 5 6]

for m in $y {
    echo $x -- $m
}

```

prints 

```text
a -- 4
a -- 5
a -- 6
```


Changelog
---------

### v0.1.3

Now allows comments "#" to end a line "##" to continue a line"
Fix so "\\\n" doesn't insert a new line into output

