# Feed Riff a number of lines output from some diff-like tool (diff,
# diff3, git diff, ...) and it will highlight that output for you.
class Riff
  DIFF_HEADER = /^diff /
  DIFF_HUNK_HEADER = /^@@ /

  ADDED = /^\+ /
  REMOVED = /^- /
  CONTEXT = /^ /

  ESC = 27.chr
  R = "#{ESC}[7m"  # REVERSE
  N = "#{ESC}[27m" # NORMAL

  BOLD = "#{ESC}[1m"
  CYAN = "#{ESC}[36m"
  GREEN = "#{ESC}[32m"
  RED = "#{ESC}[31m"

  RESET = "#{ESC}[0m"

  LINE_PREFIX = {
    initial:          '',
    diff_header:      BOLD,
    diff_hunk_header: CYAN,
    diff_hunk:        '',
    diff_add:         GREEN,
    diff_remove:      RED,
    diff_context:     ''
  }

  def initialize()
    @state = :initial
  end

  def handle_initial_line(line)
    if line =~ DIFF_HEADER
      @state = :diff_header
    end
  end

  def handle_diff_header_line(line)
    if line =~ DIFF_HUNK_HEADER
      @state = :diff_hunk_header
    end
  end

  def handle_diff_hunk_header_line(_line)
    @state = :diff_hunk
  end

  def handle_diff_hunk_line(line)
    case line
    when DIFF_HUNK_HEADER
      @state = :diff_hunk_header
    when DIFF_HEADER
      @state = :diff_header
    end
  end

  def handle_diff_line(line)
    method_name = "handle_#{@state}_line"
    fail "Unknown state: <:#{@state}>" unless
      self.respond_to? method_name

    send(method_name, line)

    puts "#{RESET}#{LINE_PREFIX.fetch(@state)}#{line}"
  end
end
