# Riff, the Refining Diff

Riff is a wrapper around diff that highlights not only which lines
have changed, but also which parts of the lines that have changed.

# Usage

```
git diff | riff
```

Or if you do...

```
git config --global pager.diff riff
git config --global pager.show riff
```

... then all future `git diff`s and `git show`s will be refined.

# FIXME: Installation

FIXME: How to place the binary in the PATH?

Optionally followed by...

```
git config --global pager.diff riff
git config --global pager.show riff
```

... to make git show refined diffs by default.

# TODO

## Before setting the new riff as `$PAGER`

- Visualize removed linefeed at end of file properly
- Visualize adding a missing linefeed at end of file properly
- Visualize missing linefeed at end of file as part of the context
  properly
- Refine `ax`->`bx\nc` properly
- Strip all color from the input before handling it to enable users to
  set Git's pager.diff and pager.show variables to 'riff' without also
  needing to set color.diff=false.
- If stdout is a terminal, pipe the output to a pager using the
  algorithm described under `core.pager` in `git help config`.
- Do some effort to prevent fork loops if people set riff as `$PAGER`
- You can do `git diff | riff` and get reasonable output.

- Refine by word rather than by character

## Before releasing the Rust version as the official one

- On exceptions, print the riff.rb @state
- On exceptions, print the line riff.rb was processing
- On exceptions, print a link to the issue tracker
- Add support for --help
- Print help and bail if stdin is a terminal
- Add support for `--version`
- On exceptions, print the current version just like `--version`
- Do not highlight anything if there are "too many" differences between the
  sections. The point here is that we want to highlight changes, but if it's a
  _replacement_ rather than a change then we don't want to highlight it.
- Put an upper bound on how large regions we should attempt to refine
- Find out how the LCS algorithm scales and improve the heuristic for
  when not to call it.

## Misc

- Add a trailing whitespace analysis pass to the Refiner
- Let the Refiner highlight whitespace errors among the added lines in
  reverse red.
- Add highlighting of non-leading tabs to the whitespace analysis
- Add test for never changing the number of lines in the input, that
  messes up `git add -p` behavior.
- Make a first public release
- Do `git show 0f5dd84` and think about how to visualize one line
  changing to itself with a comma at the end plus a bunch of entirely
  new lines. Think of a constant array getting one or more extra
  members.
- Do `git show -b 77c8f77` and think about what rule we should use to
  highlight the leading spaces of the `+ refined` and `+ page` lines
  at the end of the file.
- Don't use --dirty for the gemspec version
- Rakefile: Refuse to package dirty sources
- Make sure we can:
  - test dirty sources
  - not package dirty sources
  - package clean sources, dependencies not verified
- When special highighting an expansion, highlight the added parts in green
  reverse, rather than black reverse. Testcase: `git show 7ea6877`
- Handle plain non-git diff files
- Think about highlighting whitespace errors like Git does
- Make DiffString.add() take a color as well
- Think about how to visualize an added line break together with some
  indentation on the following line.
- Do `git show 57f27da` and think about what rule we should use to get
  the REVERSE vs reversed() lines highlighted.
- Do `git show 2ac5b06` and think about what rule we should use to
  highlight all of both `some` and `one or`.
- Make sure we highlight the output of `git log -p` properly. If we
  get something unexpected, maybe just go back to :initial?
- Make sure we highlight the output of `git show --stat` properly
- Make sure we can handle a git conflict
  resolution diff. File format is described at
  http://git-scm.com/docs/git-diff#_combined_diff_format.
- Given two files on the command line, we should pass them and any
  options on to `diff` and highlight the result.
- Given three files on the command line, we should pass them and any
  options on to `diff3` and highlight the result

# TODO future

- Detect moved blocks and use a number as a prefix for both the add
  and the remove part of the move. Hightlight any changes just like
  for other changes.

# DONE

- Make a main program that can read input from stdin and print it to
  stdout.
- Make the main program identify different kinds of lines by prefix
  and color them accordingly. Use the same color scheme as `git`.
- Make the main program identify blocks of lines that have been
  replaced by another block of lines.
- Make the Refiner not highlight anything if either old or new is
  empty
- Use <https://crates.io/crates/diffus> to refine hunks
- Build refined hunks and print them
- Highlight `^diff`, `^index`, `^+++` and `^---` lines in bold white
- Prefix all added / removed lines with the correct ANSI color code
- Don't highlight the initial `+` / `-` on added / removed lines
- Make sure we get the linefeeds right in diffs, try
  `git show 28e074bd0fc246d1caa3738432806a94f6773185` with and without `riff`.
- Visualize added line endings
- Visualize removed line endings
