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
  REFINEMENT_THRESHOLD=30

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

  def initialize(old, new)
    old_highlights = Set.new
    new_highlights = Set.new
    if (!old.empty?) && (!new.empty?)
      collect_highlights(Diff::LCS.diff(old.chomp, new.chomp),
                         old_highlights,
                         new_highlights)

      censor_highlights(old.chomp, new.chomp, old_highlights, new_highlights)
    end

    @refined_old = DiffString.new('-', RED)
    old.each_char.with_index do |char, index|
      @refined_old.add(char, old_highlights.include?(index))
    end

    @refined_new = DiffString.new('+', GREEN)
    new.each_char.with_index do |char, index|
      @refined_new.add(char, new_highlights.include?(index))
    end
  end
end
