# Feed Riff a number of lines output from some diff-like tool (diff,
# diff3, git diff, ...) and it will highlight that output for you.
class Riff
  def handle_diff_line(line)
    puts line
  end
end
