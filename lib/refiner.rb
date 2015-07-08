require 'set'
require 'diff/lcs'
require 'diff_string'

require 'colors'
require 'whitespace_lint'

# Compute longest common substring based diff between two strings.
#
# The diff format is first the old string:
# * in red
# * with each line prefixed with minuses
# * removed characters highlighted in inverse video
#
# Then comes the new string:
# * in green
# * with each line prefixed with plusses
# * added characters highlighted in inverse video
class Refiner
  include Colors
  include WhitespaceLint

  attr_reader :refined_old
  attr_reader :refined_new

  # If either old or new would get more than this percentage of chars
  # highlighted, consider this to be a replacement rather than a
  # change and just don't highlight anything.
  REFINEMENT_THRESHOLD = 30

  def collect_highlights(diff, old_highlights, new_highlights)
    diff.each do |section|
      section.each do |highlight|
        case highlight.action
        when '-'
          old_highlights << highlight.position
        when '+'
          new_highlights << highlight.position
        else
          fail("Unsupported diff action: <#{action}>")
        end
      end
    end
  end

  def censor_highlights(old, new, old_highlights, new_highlights)
    old_highlights_percentage = 100 * old_highlights.size / old.length
    new_highlights_percentage = 100 * new_highlights.size / new.length

    if old_highlights_percentage > REFINEMENT_THRESHOLD \
       || new_highlights_percentage > REFINEMENT_THRESHOLD
      # We'll consider this a replacement rather than a change, don't
      # highlight it.
      old_highlights.clear
      new_highlights.clear
    end
  end

  def should_highlight?(old, new)
    return false if old.empty? || new.empty?

    # The 15_000 constant has been determined using the "benchmark"
    # program in our bin/ directory.
    return false if old.length + new.length > 15_000

    return true
  end

  def try_highlight(old, new)
    old_highlights = Set.new
    new_highlights = Set.new
    if should_highlight?(old, new)
      collect_highlights(Diff::LCS.diff(old, new),
                         old_highlights,
                         new_highlights)

      censor_highlights(old, new, old_highlights, new_highlights)
    end

    return old_highlights, new_highlights
  end

  def try_highlight_initial_lines(old, new)
    old_line_count = old.lines.count
    new_line_count = new.lines.count
    if old_line_count == new_line_count
      return Set.new, Set.new
    end

    min_line_count = [old_line_count, new_line_count].min
    if min_line_count == 0
      return Set.new, Set.new
    end

    # Truncate old and new so they have the same number of lines
    old = old.lines[0..(min_line_count - 1)].join
    new = new.lines[0..(min_line_count - 1)].join

    return try_highlight(old, new)
  end

  # ws: a set containing the whitespace errors we want to highlight
  def render_refinement(prefix, base_color, string, highlights,
                        base_index: 0, highlight_color: '',
                        ws: nil)
    return_me = DiffString.new(prefix, base_color)
    string.each_char.with_index do |char, index|
      highlight = highlights.include?(index + base_index)
      color = highlight ? highlight_color : ''

      if !ws.nil? && ws.include?(index + base_index)
        # Highlight whitespace error in inverse red
        color = RED
        highlight = true
      end

      return_me.add(char, highlight, color)
    end
    return return_me.to_s
  end

  # After returning from this method, both @refined_old and @refined_new must
  # have been set to reasonable values.
  def create_refinements(old, new,
                         old_highlights, new_highlights,
                         whitespace_highlights)
    @refined_old = render_refinement('-', RED, old, old_highlights)
    @refined_new = render_refinement('+', GREEN,
                                     new, new_highlights,
                                     ws: whitespace_highlights)
  end

  # After returning from this method, both @refined_old and @refined_new must
  # have been set to reasonable values.
  #
  # Returns false if the preconditions for using this method aren't fulfilled
  def create_one_to_many_refinements(old, new,
                                     old_highlights, new_highlights,
                                     whitespace_highlights)
    # If things have been removed from the first line, the specialized
    # highlighting won't work
    return false if old_highlights.count > 0

    # If the first line was replaced rather than updated, the specialized
    # highlighting won't work
    return false if new_highlights.count == 0

    # Specialized highlighting requires exactly one old line
    return false if old.lines.count != 1

    lines = new.lines
    # Specialized highlighting requires two or more new lines
    return false if lines.count < 2

    @refined_old = ''

    refined_line_1 =
      render_refinement(' ', '', lines[0], new_highlights,
                        highlight_color: GREEN,
                        ws: whitespace_highlights)

    line_2_index_0 = lines[0].length
    refined_remaining_lines = render_refinement('+', GREEN,
                                                lines[1..-1].join,
                                                new_highlights,
                                                base_index: line_2_index_0,
                                                ws: whitespace_highlights)

    @refined_new = refined_line_1 + refined_remaining_lines

    return true
  end

  def initialize(old, new)
    old_highlights, new_highlights = try_highlight(old, new)
    if old_highlights.size == 0 && new_highlights.size == 0
      old_highlights, new_highlights = try_highlight_initial_lines(old, new)
    end

    whitespace_highlights = collect_ws_highlights(new)

    if !create_one_to_many_refinements(old, new,
                                       old_highlights,
                                       new_highlights,
                                       whitespace_highlights)
      create_refinements(old, new,
                         old_highlights, new_highlights,
                         whitespace_highlights)
    end
  end
end
