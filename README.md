# Riff, the Refining Diff

Riff is a wrapper around diff that highlights not only which lines
have changed, but also which parts of the lines that have changed.

## Algorithm

The refinement algorithm will be inspired by
<https://en.wikipedia.org/wiki/Longest_common_subsequence_problem#Print_the_diff>.

## Minimum Viable Product

You can do `git diff | riff` and get reasonable output.

# TODO
* Diffing <x "hej"> vs <x 'hej'> shows the first space as a
difference.
* Maybe use http://www.rubydoc.info/github/halostatue/diff-lcs rather
than our own refinement algorithm?
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
