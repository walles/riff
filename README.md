# Riff, the Refining Diff
Riff is a wrapper around diff that highlights not only which lines
have changed, but also which parts of the lines that have changed.

# Usage
    git diff | riff

Or if you do...

    git config --global pager.diff riff
    git config --global pager.show riff
    git config --global color.diff false

... then all future 'git diff's and 'git show's will be refined.

## Minimum Viable Product
You can do `git diff | riff` and get reasonable output.

# TODO before first release
* Make the Refiner not highlight anything if there are "too many"
differences between the sections. The point here is that we want to
highlight changes, but if it's a *replacement* rather than a change
then we don't want to highlight it.
* Refine "ax"->"bx\nc" properly
* Refine added line endings properly
* Refine removed line endings properly
* Handle missing linefeed at end of file properly
* Test that we work as expected when "gem install"ed system-wide
* On exceptions, print a link to the issue tracker
* Add support for --help
* Add support for --version
* Release version 0.0.0

# TODO post first release
* Make sure we highlight the output of "git log -p" properly. If we
get something unexpected, maybe just go back to :initial?
* Make sure we highlight the output of "git show --stat" properly
* Somehow hint users on what to write in their .gitconfig to make
selected commands page through riff
* Strip all color from the input before handling it to enable users to
  set Git's pager.diff and pager.show variables to 'riff' without also
  needing to set color.diff=false.
* Given two files on the command line, we should pass them and any
options on to "diff" and highlight the result.
* Given three files on the command line, we should pass them and any
options on to "diff3" and highlight the result

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
