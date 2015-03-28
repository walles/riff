# Feed Riff a number of lines output from some diff-like tool (diff,
# diff3, git diff, ...) and it will highlight that output for you.
class Riff
  DIFF_HEADER = /^diff /
  DIFF_HUNK_HEADER = /^@@ /

  def initialize()
    @state = :initial
  end

  def handle_diff_line(line)
    if @state == :initial
      if line =~ DIFF_HEADER
        @state = :diff_header
      end
    elsif @state == :diff_header
      if line =~ DIFF_HUNK_HEADER
        @state = :diff_hunk_header
      end
    elsif @state == :diff_hunk_header
      @state = :diff_hunk
    elsif @state == :diff_hunk
      if line =~ DIFF_HUNK_HEADER
        @state = :diff_hunk_header
      elsif line =~ DIFF_HEADER
        @state = :diff_header
      end
    else
      fail "Unknown state: #{@state}"
    end

    puts "<#{@state}>#{line}"
  end
end
