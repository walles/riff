# Riff, the Refining Diff
Riff is a wrapper around diff that highlights not only which lines
have changed, but also which parts of the lines that have changed.

## Minimum Viable Product
You can do `git diff | riff` and get reasonable output.

# TODO
* Make the Refiner not highlight anything if either old or new is
empty
* Ask the Refiner even if either old or new is empty
* Use DiffString for context lines?
* Refine added line endings properly
* Refine removed line endings properly
* Handle missing linefeed at end of file properly
* Preserve linefeeds when sending lines to the Refiner
* All context lines must be prefixed by ' ', currently they aren't
* Diffing <x "hej"> vs <x 'hej'> shows the first space as a
difference.
* Refine each pair of blocks, make sure both added characters and
  removed characters are highlighted in a readable fashion, both in
  added blocks and removed blocks.
* Test that we work as expected when "gem install"ed system-wide
* See if we can get Riff to transparently insert itself as "git riff"
  and treat that as git diff | riff | $PAGER.

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
