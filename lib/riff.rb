require 'colors'
require 'refiner'

# Call do_stream() with the output of some diff-like tool (diff,
# diff3, git diff, ...) and it will highlight that output for you.
class Riff
  DIFF_HEADER = /^diff /
  DIFF_HUNK_HEADER = /^@@ /

  DIFF_ADDED = /^\+(.*)/
  DIFF_REMOVED = /^-(.*)/
  DIFF_CONTEXT = /^ /

  include Colors

  LINE_PREFIX = {
    initial:          '',
    diff_header:      BOLD,
    diff_hunk_header: CYAN,
    diff_hunk:        '',
    diff_added:       GREEN,
    diff_removed:     RED,
    diff_context:     ''
  }

  def initialize()
    @state = :initial

    @replace_old = ""
    @replace_new = ""
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

  def handle_diff_hunk_header_line(line)
    handle_diff_hunk_line(line)
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

  # Highlight differences between @replace_old and @replace_new
  def print_refined_diff(old, new)
    refiner = Refiner.new(old, new)
    puts refiner.refined_old
    puts refiner.refined_new
  end

  # If we have stored adds / removes, calling this method will flush
  # those.
  def consume_replacement()
    return if @replace_old.empty? && @replace_new.empty?

    if @replace_new.empty?
      style = LINE_PREFIX.fetch(:diff_removed)
      @replace_old.lines.each { |line| print_styled_line(style, line.chomp) }
    elsif @replace_old.empty?
      style = LINE_PREFIX.fetch(:diff_added)
      @replace_new.lines.each { |line| print_styled_line(style, line.chomp) }
    else
      print_refined_diff(@replace_old, @replace_new)
    end

    @replace_old = ''
    @replace_new = ''
  end

  def print_styled_line(style, line)
    reset = (style.empty? ? '' : RESET)
    puts "#{style}#{line}#{reset}"
  end

  def handle_diff_line(line)
    line.chomp!

    method_name = "handle_#{@state}_line"
    fail "Unknown state: <:#{@state}>" unless
      self.respond_to? method_name

    send(method_name, line)

    case @state
    when :diff_added
      @replace_new += DIFF_ADDED.match(line)[1] + "\n"
    when :diff_removed
      @replace_old += DIFF_REMOVED.match(line)[1] + "\n"
    else
      consume_replacement()

      print_styled_line(LINE_PREFIX.fetch(@state), line)
    end
  end

  # Read diff from a stream and output a highlighted version to stdout
  def do_stream(diff_stream)
    diff_stream.each do |line|
      handle_diff_line(line)
    end
    consume_replacement()
  end
end
