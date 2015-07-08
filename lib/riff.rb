require 'colors'
require 'refiner'
require 'diff_string'

# Call do_stream() with the output of some diff-like tool (diff,
# diff3, git diff, ...) and it will highlight that output for you.
class Riff
  DIFF_HEADER = /^diff /
  DIFF_HUNK_HEADER = /^@@ /
  DIFF_REPLACED_FILE_HEADER = /^---/

  DIFF_ADDED = /^\+(.*)/
  DIFF_REMOVED = /^-(.*)/
  DIFF_CONTEXT = /^ /
  DIFF_NO_ENDING_NEWLINE = /^\\/

  include Colors

  LINE_PREFIX = {
    initial:          '',
    diff_header:      BOLD,
    diff_hunk_header: CYAN,
    diff_hunk:        '',
    diff_added:       GREEN,
    diff_removed:     RED,
    diff_context:     '',
    diff_no_ending_newline: ''
  }

  def initialize()
    @state = :initial

    @replace_old = ''
    @replace_new = ''
  end

  def handle_initial_line(line)
    if line =~ DIFF_HEADER
      @state = :diff_header
    elsif line =~ DIFF_REPLACED_FILE_HEADER
      @state = :diff_header
    end
  end

  def handle_diff_header_line(line)
    if line =~ DIFF_HUNK_HEADER
      @state = :diff_hunk_header
    end
  end

  def handle_diff_hunk_header_line(line)
    handle_diff_hunk_line(line)
  end

  def handle_no_ending_newline(line)
    case @state
    when :diff_added
      @replace_new.sub!(/\n$/, '')
    when :diff_removed
      @replace_old.sub!(/\n$/, '')
    when :diff_context
      # Intentionally ignored
      return
    else
      fail NotImplementedError,
           "Can't handle no-ending-newline in <#{@state}> line: <#{line}>"
    end

    @state = :diff_no_ending_newline
  end

  def handle_diff_hunk_line(line)
    case line
    when DIFF_HUNK_HEADER
      @state = :diff_hunk_header
    when DIFF_HEADER
      @state = :diff_header
    when DIFF_ADDED
      @state = :diff_added
    when DIFF_REMOVED
      @state = :diff_removed
    when DIFF_CONTEXT
      @state = :diff_context
    when DIFF_NO_ENDING_NEWLINE
      handle_no_ending_newline(line)
    else
      fail NotImplementedError, "Can't handle <#{@state}> line: <#{line}>"
    end
  end

  def handle_diff_added_line(line)
    handle_diff_hunk_line(line)
  end

  def handle_diff_removed_line(line)
    handle_diff_hunk_line(line)
  end

  def handle_diff_context_line(line)
    handle_diff_hunk_line(line)
  end

  def handle_diff_no_ending_newline_line(line)
    handle_diff_hunk_line(line)
  end

  # If we have stored adds / removes, calling this method will flush
  # those.
  def consume_replacement()
    return '' if @replace_old.empty? && @replace_new.empty?

    refiner = Refiner.new(@replace_old, @replace_new)
    return_me = refiner.refined_old
    return_me += refiner.refined_new

    @replace_old = ''
    @replace_new = ''

    return return_me
  end

  # Call handle_<state>_line() for the given state and line
  def handle_line_for_state(state, line)
    method_name = "handle_#{state}_line"
    fail "Unknown state: <:#{state}>" unless
      self.respond_to? method_name

    send(method_name, line)
  end

  def handle_diff_line(line)
    line.chomp!
    line = uncolor(line)

    handle_line_for_state(@state, line)

    case @state
    when :diff_added
      @replace_new += DIFF_ADDED.match(line)[1] + "\n"
      return ''
    when :diff_removed
      @replace_old += DIFF_REMOVED.match(line)[1] + "\n"
      return ''
    when :diff_no_ending_newline
      return ''
    else
      refined = consume_replacement()

      color = LINE_PREFIX.fetch(@state)

      return refined + DiffString.decorate_string('', color, line + "\n")
    end
  end

  # Read diff from a stream and output a highlighted version to stdout
  def do_stream(diff_stream)
    output = ''
    current_line = nil

    begin
      diff_stream.each do |line|
        current_line = line
        output += handle_diff_line(line)
      end
      output += consume_replacement()
    rescue
      STDERR.puts "State: <#{@state}>"
      STDERR.puts "Current line: <#{current_line}>"
      raise
    end

    return output
  end
end
