require 'set'
require 'diff/lcs'
require 'diff_string'

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
          fail("Unsupported diff type: <#{type}>")
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

  # After returning from this method, both @refined_old and @refined_new must
  # have been set to reasonable values.
  def create_refinements(old, new, old_highlights, new_highlights)
    refined_old = DiffString.new('-', RED)
    old.each_char.with_index do |char, index|
      refined_old.add(char, old_highlights.include?(index))
    end
    @refined_old = refined_old.to_s

    refined_new = DiffString.new('+', GREEN)
    new.each_char.with_index do |char, index|
      refined_new.add(char, new_highlights.include?(index))
    end
    @refined_new = refined_new.to_s
  end

  def initialize(old, new)
    old_highlights, new_highlights = try_highlight(old, new)
    if old_highlights.size == 0 && new_highlights.size == 0
      old_highlights, new_highlights = try_highlight_initial_lines(old, new)
    end

    create_refinements(old, new, old_highlights, new_highlights)
  end
end
