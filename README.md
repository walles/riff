# Riff, the Refining Diff
Riff is a wrapper around diff that highlights not only which lines
have changed, but also which parts of the lines that have changed.

# Usage
    git diff | riff

Or if you do...

    git config --global pager.diff riff
    git config --global pager.show riff

... then all future 'git diff's and 'git show's will be refined.

## Minimum Viable Product
You can do `git diff | riff` and get reasonable output.

# TODO before first release
* Put an upper bound on how large regions we should refine
* Test that we work as expected when "gem install"ed system-wide
* Release version 0.0.0

# TODO post first release
* Think about highlighting whitespace errors like Git does
* Think about how to visualize one line changing to itself with a
  comma at the end plus a bunch of entirely new lines. Think of a
  constant array getting one or more extra members.
* Do "git show 57f27da" and think about what rule we should use to get
the REVERSE vs reversed() lines highlighted.
* Do "git show 2ac5b06" and think about what rule we should use to
highlight all of both "some" and "one or".
* Do "git show -b 77c8f77" and think about what rule we should use to
  highlight the leading spaces of the "+  refined" and "+  page" lines
  at the end of the file.
* Make sure we highlight the output of "git log -p" properly. If we
get something unexpected, maybe just go back to :initial?
* Make sure we highlight the output of "git show --stat" properly
* Make sure we can handle a git conflict
  resolution diff. File format is described at
  http://git-scm.com/docs/git-diff#_combined_diff_format.
* Given two files on the command line, we should pass them and any
options on to "diff" and highlight the result.
* Given three files on the command line, we should pass them and any
options on to "diff3" and highlight the result

# TODO future
* Detect moved blocks and use a number as a prefix for both the add
  and the remove part of the move. Hightlight any changes just like
  for other changes.

# DONE
* Make a main program that can read input from stdin and print it to
stdout.
* Make the main program identify different kinds of lines by prefix
and color them accordingly. Use the same color scheme as `git`.
* Make the main program identify blocks of lines that have been
replaced by another block of lines.
* Use http://www.rubydoc.info/github/halostatue/diff-lcs rather
than our own refinement algorithm
* Make it possible to print rather than puts Refiner output
* "print" rather than "puts" the Refiner output
* Make the Refiner not highlight anything if either old or new is
empty
* Ask the Refiner even if either old or new is empty
* Use DiffString for context lines
* Preserve linefeeds when sending lines to the Refiner
* All context lines must be prefixed by ' ', currently they aren't
* Refine each pair of blocks, make sure both added characters and
  removed characters are highlighted in a readable fashion, both in
  added blocks and removed blocks.
* Diffing <x "hej"> vs <x 'hej'> shows the first space as a
difference.
* If stdout is a terminal, pipe the output to a pager using the
algorithm described under "core.pager" in "git help config".
* Do some effort to prevent fork loops if people set riff as $PAGER
* Make the Refiner not highlight anything if there are "too many"
differences between the sections. The point here is that we want to
highlight changes, but if it's a *replacement* rather than a change
then we don't want to highlight it.
* Refine added line endings properly
* Refine removed line endings properly
* Refine "ax"->"bx\nc" properly
* Strip all color from the input before handling it to enable users to
  set Git's pager.diff and pager.show variables to 'riff' without also
  needing to set color.diff=false.
* Visualize removed linefeed at end of file properly
* Visualize adding a missing linefeed at end of file properly
* Visualize missing linefeed at end of file as part of the context
properly
* On exceptions, print the riff.rb @state
* On exceptions, print the line riff.rb was processing
* On exceptions, print a link to the issue tracker
* Add support for --help
* Print help and bail if stdin is a terminal
* Add support for --version
* On exceptions, print the current version just like --version
