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

  def initialize(old, new)
    old_highlights = Set.new
    new_highlights = Set.new
    if (!old.empty?) && (!new.empty?)
      collect_highlights(Diff::LCS.diff(old.chomp, new.chomp),
                         old_highlights,
                         new_highlights)
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
